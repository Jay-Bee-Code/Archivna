//! سيناريو جلسة المزامنة بين عقدتين. متتابع عمدًا (لا قراءة/كتابة متزامنة)
//! لتفادي أي جمود (deadlock)، ويقوده الطرف المبادر (العميل).

use crate::crypto;
use crate::sync::protocol::{Msg, Session};
use crate::sync::store::{self, SyncEvent};
use crate::sync::SyncError;
use base64::{engine::general_purpose, Engine as _};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

pub const CHUNK: usize = 192 * 1024;
pub const EVENT_BATCH: usize = 100;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct SyncCore {
    pub db: Arc<Mutex<Option<Connection>>>,
    pub vault_key: Arc<Mutex<Option<[u8; 32]>>>,
    pub vault_dir: PathBuf,
    pub sync_key: [u8; 32],
    pub node_id: String,
    pub node_name: String,
    /// جلسة مزامنة واحدة في كل مرة على هذا الجهاز (try_lock: المشغول يُرفض بدل الانتظار)
    pub session_lock: Arc<tokio::sync::Mutex<()>>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct SyncReport {
    pub peer_node_id: String,
    pub peer_name: String,
    pub events_received: usize,
    pub events_sent: usize,
    pub files_received: usize,
    pub files_sent: usize,
}

impl SyncCore {
    fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T, SyncError>) -> Result<T, SyncError> {
        let guard = self
            .db
            .lock()
            .map_err(|_| SyncError::Protocol("تعذر قفل قاعدة البيانات".into()))?;
        let conn = guard.as_ref().ok_or(SyncError::Locked)?;
        f(conn)
    }

    fn key(&self) -> Result<[u8; 32], SyncError> {
        self.vault_key
            .lock()
            .map_err(|_| SyncError::Protocol("تعذر قفل مفتاح الخزنة".into()))?
            .ok_or(SyncError::Locked)
    }
}

// ------------------------------------------------------------------- العميل

pub async fn run_client(core: &SyncCore, addr: SocketAddr) -> Result<SyncReport, SyncError> {
    let _guard = core.session_lock.try_lock().map_err(|_| SyncError::Busy)?;

    let (ws, _) = tokio::time::timeout(
        CONNECT_TIMEOUT,
        tokio_tungstenite::connect_async(format!("ws://{}", addr)),
    )
    .await
    .map_err(|_| SyncError::Timeout)??;
    let mut s = Session::new(ws, core.sync_key, true);

    let challenge = crypto::random_hex(16);
    s.send(Msg::Hello {
        node_id: core.node_id.clone(),
        node_name: core.node_name.clone(),
        challenge: challenge.clone(),
    })
    .await?;

    let (peer_id, peer_name) = match s.recv().await {
        Ok(Msg::HelloAck { node_id, node_name, echo, challenge: server_challenge }) => {
            if echo != challenge {
                return Err(SyncError::Auth);
            }
            s.send(Msg::Confirm { echo: server_challenge }).await?;
            (node_id, node_name)
        }
        Ok(_) => return Err(SyncError::Protocol("رد غير متوقع في المصافحة".into())),
        Err(SyncError::Peer(m)) => return Err(SyncError::Peer(m)),
        // الطرف الآخر يقطع الاتصال حين لا يفك تشفير رسالتنا، أي أن المفتاح مختلف
        Err(_) => return Err(SyncError::Auth),
    };
    if peer_id == core.node_id {
        return Err(SyncError::Protocol("هذا الجهاز نفسه".into()));
    }

    let mut report = exchange(core, &mut s, true).await?;
    report.peer_node_id = peer_id;
    report.peer_name = peer_name;
    Ok(report)
}

// ------------------------------------------------------------------- الخادم

pub async fn handle_incoming(core: SyncCore, stream: TcpStream) -> Result<SyncReport, SyncError> {
    let ws = tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::accept_async(stream))
        .await
        .map_err(|_| SyncError::Timeout)??;
    let mut s = Session::new(ws, core.sync_key, false);

    let (peer_id, peer_name, challenge) = match s.recv().await? {
        Msg::Hello { node_id, node_name, challenge } => (node_id, node_name, challenge),
        _ => return Err(SyncError::Protocol("كان متوقعًا Hello".into())),
    };
    let my_challenge = crypto::random_hex(16);
    s.send(Msg::HelloAck {
        node_id: core.node_id.clone(),
        node_name: core.node_name.clone(),
        echo: challenge,
        challenge: my_challenge.clone(),
    })
    .await?;
    match s.recv().await? {
        Msg::Confirm { echo } if echo == my_challenge => {}
        _ => return Err(SyncError::Auth),
    }
    if peer_id == core.node_id {
        return Err(SyncError::Protocol("اتصال من الجهاز نفسه".into()));
    }

    let _guard = match core.session_lock.try_lock() {
        Ok(g) => g,
        Err(_) => {
            let _ = s.send(Msg::Error { message: "busy".into() }).await;
            return Err(SyncError::Busy);
        }
    };

    let mut report = exchange(&core, &mut s, false).await?;
    report.peer_node_id = peer_id;
    report.peer_name = peer_name;
    Ok(report)
}

/// حلقة قبول الاتصالات الواردة. `on_done` تُستدعى بنتيجة كل جلسة واردة.
pub async fn serve(
    core: SyncCore,
    listener: TcpListener,
    mut shutdown: watch::Receiver<bool>,
    on_done: Arc<dyn Fn(Result<SyncReport, SyncError>) + Send + Sync>,
) {
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else { continue };
                let core = core.clone();
                let cb = on_done.clone();
                tokio::spawn(async move {
                    cb(handle_incoming(core, stream).await);
                });
            }
            _ = shutdown.changed() => break,
        }
    }
}

// ---------------------------------------------------------- سيناريو التبادل

async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(
    core: &SyncCore,
    s: &mut Session<S>,
    is_client: bool,
) -> Result<SyncReport, SyncError> {
    let mut report = SyncReport::default();

    // 1) تبادل متجهات الإصدار
    let mine = core.with_conn(|c| store::version_vector(c))?;
    let peer_vec = if is_client {
        s.send(Msg::Versions { vector: mine }).await?;
        expect_versions(s).await?
    } else {
        let v = expect_versions(s).await?;
        s.send(Msg::Versions { vector: mine }).await?;
        v
    };

    // 2) الأحداث: العميل يرسل أولًا ثم يستقبل، والخادم العكس
    if is_client {
        report.events_sent = send_events(core, s, &peer_vec).await?;
        report.events_received = recv_events(core, s).await?;
    } else {
        report.events_received = recv_events(core, s).await?;
        report.events_sent = send_events(core, s, &peer_vec).await?;
    }

    // 3) الملفات: من يطلب أولًا هو العميل
    // 4) إنهاء بتأكيد متبادل: لا يعدّ المبادر الجلسة مكتملة قبل أن يؤكد الطرف الآخر
    //    أنه انتهى من كتابة كل ما استلمه.
    if is_client {
        report.files_received = request_files(core, s).await?;
        report.files_sent = serve_files(core, s).await?;
        s.send(Msg::Bye).await?;
        expect_bye(s).await?;
    } else {
        report.files_sent = serve_files(core, s).await?;
        report.files_received = request_files(core, s).await?;
        expect_bye(s).await?;
        s.send(Msg::Bye).await?;
    }
    Ok(report)
}

async fn expect_bye<S: AsyncRead + AsyncWrite + Unpin>(s: &mut Session<S>) -> Result<(), SyncError> {
    match s.recv().await? {
        Msg::Bye => Ok(()),
        _ => Err(SyncError::Protocol("كان متوقعًا Bye".into())),
    }
}

async fn expect_versions<S: AsyncRead + AsyncWrite + Unpin>(
    s: &mut Session<S>,
) -> Result<HashMap<String, i64>, SyncError> {
    match s.recv().await? {
        Msg::Versions { vector } => Ok(vector),
        _ => Err(SyncError::Protocol("كان متوقعًا Versions".into())),
    }
}

async fn send_events<S: AsyncRead + AsyncWrite + Unpin>(
    core: &SyncCore,
    s: &mut Session<S>,
    peer_vec: &HashMap<String, i64>,
) -> Result<usize, SyncError> {
    let events: Vec<SyncEvent> = core.with_conn(|c| store::events_missing(c, peer_vec))?;
    for batch in events.chunks(EVENT_BATCH) {
        s.send(Msg::Events { events: batch.to_vec() }).await?;
    }
    s.send(Msg::EventsEnd).await?;
    Ok(events.len())
}

async fn recv_events<S: AsyncRead + AsyncWrite + Unpin>(
    core: &SyncCore,
    s: &mut Session<S>,
) -> Result<usize, SyncError> {
    let mut total = 0;
    loop {
        match s.recv().await? {
            Msg::Events { events } => {
                total += core.with_conn(|c| store::apply_remote_events(c, &events, &core.vault_dir))?;
            }
            Msg::EventsEnd => return Ok(total),
            _ => return Err(SyncError::Protocol("كان متوقعًا Events".into())),
        }
    }
}

async fn request_files<S: AsyncRead + AsyncWrite + Unpin>(
    core: &SyncCore,
    s: &mut Session<S>,
) -> Result<usize, SyncError> {
    let missing = core.with_conn(|c| store::missing_files(c, &core.vault_dir))?;
    s.send(Msg::NeedFiles { hashes: missing.clone() }).await?;
    let key = core.key()?;

    let mut partial: HashMap<String, (Vec<u8>, u32)> = HashMap::new();
    let mut received = 0;
    loop {
        match s.recv().await? {
            Msg::FileChunk { hash, index, total, data } => {
                // لا نقبل إلا ما طلبناه فعلًا
                if !missing.contains(&hash) {
                    return Err(SyncError::Protocol("ملف لم يُطلب".into()));
                }
                let bytes = general_purpose::STANDARD
                    .decode(data)
                    .map_err(|_| SyncError::Protocol("base64 غير صالح".into()))?;
                let entry = partial.entry(hash.clone()).or_insert_with(|| (Vec::new(), 0));
                if index != entry.1 {
                    return Err(SyncError::Protocol("ترتيب أجزاء الملف غير صحيح".into()));
                }
                entry.0.extend_from_slice(&bytes);
                entry.1 += 1;
                if entry.1 == total {
                    let (buf, _) = partial.remove(&hash).unwrap();
                    // ملف تالف/غير مطابق لبصمته يُتجاهل، ويُعاد طلبه في الجلسة التالية
                    if store::write_blob_from_plain(&core.vault_dir, &key, &hash, &buf).is_ok() {
                        received += 1;
                    }
                }
            }
            Msg::FilesEnd => return Ok(received),
            _ => return Err(SyncError::Protocol("كان متوقعًا FileChunk".into())),
        }
    }
}

async fn serve_files<S: AsyncRead + AsyncWrite + Unpin>(
    core: &SyncCore,
    s: &mut Session<S>,
) -> Result<usize, SyncError> {
    let hashes = match s.recv().await? {
        Msg::NeedFiles { hashes } => hashes,
        _ => return Err(SyncError::Protocol("كان متوقعًا NeedFiles".into())),
    };
    let key = core.key()?;
    let mut sent = 0;
    for hash in hashes {
        let Ok(plain) = store::read_blob_plain(&core.vault_dir, &key, &hash) else {
            continue; // لا نملكه (أو hash غير صالح)
        };
        let total = if plain.is_empty() { 1 } else { plain.len().div_ceil(CHUNK) } as u32;
        if plain.is_empty() {
            s.send(Msg::FileChunk { hash: hash.clone(), index: 0, total, data: String::new() }).await?;
        } else {
            for (i, chunk) in plain.chunks(CHUNK).enumerate() {
                s.send(Msg::FileChunk {
                    hash: hash.clone(),
                    index: i as u32,
                    total,
                    data: general_purpose::STANDARD.encode(chunk),
                })
                .await?;
            }
        }
        sent += 1;
    }
    s.send(Msg::FilesEnd).await?;
    Ok(sent)
}
