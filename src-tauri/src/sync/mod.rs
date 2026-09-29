//! محرك المزامنة (Phase 4): اكتشاف عبر mDNS + WebSocket مشفَّر + سجل أحداث.
//! هذه الوحدة لا تعتمد على Tauri؛ الربط بالتطبيق في commands/sync.rs.

pub mod discovery;
pub mod net;
pub mod protocol;
pub mod store;

use mdns_sd::ServiceDaemon;
use net::{SyncCore, SyncReport};
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{watch, Notify};

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("خطأ قاعدة بيانات: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("خطأ في الملفات: {0}")]
    Io(#[from] std::io::Error),
    #[error("خطأ اتصال: {0}")]
    Ws(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("خطأ في البيانات: {0}")]
    Json(#[from] serde_json::Error),
    #[error("فشلت المصادقة — مفتاح المزامنة غير مطابق على الأرجح")]
    Auth,
    #[error("الخزنة مقفلة")]
    Locked,
    #[error("الجهاز مشغول بمزامنة أخرى")]
    Busy,
    #[error("انتهت مهلة الاتصال")]
    Timeout,
    #[error("أُغلق الاتصال")]
    Closed,
    #[error("{0}")]
    Protocol(String),
    #[error("رفض الجهاز الآخر: {0}")]
    Peer(String),
}

#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub node_id: String,
    pub name: String,
    pub addrs: Vec<SocketAddr>,
    pub last_seen: i64,
    pub last_sync: Option<i64>,
    pub last_error: Option<String>,
}

pub type Peers = Arc<Mutex<HashMap<String, PeerInfo>>>;

#[derive(Serialize, Clone, Debug)]
pub struct PeerView {
    pub node_id: String,
    pub name: String,
    pub addr: String,
    pub last_seen: i64,
    pub last_sync: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct EngineStatus {
    pub node_id: String,
    pub node_name: String,
    pub port: u16,
    pub peers: Vec<PeerView>,
    pub last_sync: Option<i64>,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// مقبض التحكم بالمحرك (يُحفظ في حالة Tauri).
pub struct SyncEngine {
    pub node_id: String,
    pub node_name: String,
    pub port: u16,
    peers: Peers,
    last_sync: Arc<Mutex<Option<i64>>>,
    trigger: Arc<Notify>,
    shutdown_tx: watch::Sender<bool>,
    daemon: ServiceDaemon,
}

/// الجزء الذي يعمل في الخلفية (يُمرَّر لـ spawn).
pub struct EngineRunner {
    core: SyncCore,
    listener: std::net::TcpListener,
    peers: Peers,
    last_sync: Arc<Mutex<Option<i64>>>,
    trigger: Arc<Notify>,
    discovered: Arc<Notify>,
    shutdown_rx: watch::Receiver<bool>,
    daemon: ServiceDaemon,
}

/// حالة Tauri المُدارة: المحرك (إن كان مفعّلًا).
pub struct SyncState(pub Mutex<Option<SyncEngine>>);

impl SyncState {
    /// يطلب مزامنة فورية بعد تغيير محلي (لا يفعل شيئًا إن كان المحرك متوقفًا).
    pub fn poke(&self) {
        if let Ok(g) = self.0.lock() {
            if let Some(e) = g.as_ref() {
                e.trigger.notify_one();
            }
        }
    }
}

/// يجهّز المحرك: يربط منفذًا عشوائيًا ويعلن عن الجهاز عبر mDNS. متزامن (بلا async).
pub fn prepare(core: SyncCore) -> Result<(SyncEngine, EngineRunner), SyncError> {
    let listener = std::net::TcpListener::bind("0.0.0.0:0")?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();

    let daemon = ServiceDaemon::new().map_err(|e| SyncError::Protocol(format!("mDNS: {e}")))?;
    discovery::advertise(&daemon, &core.node_id, &core.node_name, port)?;

    let peers: Peers = Arc::new(Mutex::new(HashMap::new()));
    let last_sync = Arc::new(Mutex::new(None));
    let trigger = Arc::new(Notify::new());
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let engine = SyncEngine {
        node_id: core.node_id.clone(),
        node_name: core.node_name.clone(),
        port,
        peers: peers.clone(),
        last_sync: last_sync.clone(),
        trigger: trigger.clone(),
        shutdown_tx,
        daemon: daemon.clone(),
    };
    let discovered = Arc::new(Notify::new());
    let runner = EngineRunner { core, listener, peers, last_sync, trigger, discovered, shutdown_rx, daemon };
    Ok((engine, runner))
}

impl SyncEngine {
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
        let _ = self.daemon.shutdown();
    }

    pub fn trigger_now(&self) {
        self.trigger.notify_one();
    }

    pub fn status(&self) -> EngineStatus {
        let peers = self
            .peers
            .lock()
            .map(|m| {
                let mut v: Vec<PeerView> = m
                    .values()
                    .map(|p| PeerView {
                        node_id: p.node_id.clone(),
                        name: p.name.clone(),
                        addr: p.addrs.first().map(|a| a.to_string()).unwrap_or_default(),
                        last_seen: p.last_seen,
                        last_sync: p.last_sync,
                        last_error: p.last_error.clone(),
                    })
                    .collect();
                v.sort_by(|a, b| a.name.cmp(&b.name));
                v
            })
            .unwrap_or_default();
        EngineStatus {
            node_id: self.node_id.clone(),
            node_name: self.node_name.clone(),
            port: self.port,
            peers,
            last_sync: self.last_sync.lock().ok().and_then(|g| *g),
        }
    }
}

const ROUND_INTERVAL: Duration = Duration::from_secs(15);

impl EngineRunner {
    pub async fn run(self) {
        let EngineRunner { core, listener, peers, last_sync, trigger, discovered, mut shutdown_rx, daemon } = self;

        let Ok(listener) = TcpListener::from_std(listener) else { return };

        // نتيجة الجلسات الواردة: نحدّث آخر مزامنة والجهاز المقابل
        let cb_peers = peers.clone();
        let cb_last = last_sync.clone();
        let on_done: Arc<dyn Fn(Result<SyncReport, SyncError>) + Send + Sync> =
            Arc::new(move |res| record_result(&cb_peers, &cb_last, None, res));
        tokio::spawn(net::serve(core.clone(), listener, shutdown_rx.clone(), on_done));

        if let Ok(rx) = discovery::browse(&daemon) {
            tokio::spawn(discovery::watch(
                rx,
                peers.clone(),
                core.node_id.clone(),
                discovered.clone(),
                shutdown_rx.clone(),
            ));
        }

        let mut force = false;
        loop {
            sync_round(&core, &peers, &last_sync, force).await;
            tokio::select! {
                _ = tokio::time::sleep(ROUND_INTERVAL) => { force = false; }
                _ = trigger.notified() => {
                    // تجميع التغييرات المتتالية قبل الانطلاق
                    tokio::time::sleep(Duration::from_millis(800)).await;
                    force = true;
                }
                _ = discovered.notified() => {
                    // جهاز جديد: جولة عادية (المبادر هو صاحب المعرّف الأصغر فقط)
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    force = false;
                }
                _ = shutdown_rx.changed() => break,
            }
        }
    }
}

fn record_result(
    peers: &Peers,
    last_sync: &Arc<Mutex<Option<i64>>>,
    peer_hint: Option<&str>,
    res: Result<SyncReport, SyncError>,
) {
    let t = now();
    match res {
        Ok(rep) => {
            if let Ok(mut g) = last_sync.lock() {
                *g = Some(t);
            }
            if let Ok(mut map) = peers.lock() {
                if let Some(p) = map.get_mut(&rep.peer_node_id) {
                    p.last_sync = Some(t);
                    p.last_error = None;
                }
            }
        }
        Err(SyncError::Busy) => {}
        // الطرف الآخر مشغول بجلسة أخرى: عابر، وسيُعاد المحاولة في الجولة التالية
        Err(SyncError::Peer(ref m)) if m == "busy" => {}
        Err(e) => {
            if let (Some(id), Ok(mut map)) = (peer_hint, peers.lock()) {
                if let Some(p) = map.get_mut(id) {
                    p.last_error = Some(e.to_string());
                }
            }
        }
    }
}

/// جولة مزامنة مع الأجهزة المكتشفة. في الجولات الدورية يبادر الجهاز صاحب المعرّف
/// الأصغر فقط (لتفادي اتصالين متقابلين)؛ وعند الطلب الفوري (force) نبادر للجميع.
async fn sync_round(
    core: &SyncCore,
    peers: &Peers,
    last_sync: &Arc<Mutex<Option<i64>>>,
    force: bool,
) {
    let snapshot: Vec<PeerInfo> = peers.lock().map(|m| m.values().cloned().collect()).unwrap_or_default();
    for peer in snapshot {
        if !force && core.node_id >= peer.node_id {
            continue;
        }
        let mut last: Option<Result<SyncReport, SyncError>> = None;
        for addr in &peer.addrs {
            match net::run_client(core, *addr).await {
                Ok(r) => {
                    last = Some(Ok(r));
                    break;
                }
                Err(e) => last = Some(Err(e)),
            }
        }
        if let Some(res) = last {
            record_result(peers, last_sync, Some(&peer.node_id), res);
        }
    }
}
