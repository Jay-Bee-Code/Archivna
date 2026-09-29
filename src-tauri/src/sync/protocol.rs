//! بروتوكول المزامنة: رسائل JSON مغلَّفة ومشفّرة بـ AES-256-GCM (مفتاح مزامنة المؤسسة)
//! فوق WebSocket. كل رسالة تحمل عدّادًا واتجاهًا للحماية من إعادة الإرسال والانعكاس.

use crate::crypto;
use crate::sync::store::SyncEvent;
use crate::sync::SyncError;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

pub const RECV_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "t")]
pub enum Msg {
    Hello { node_id: String, node_name: String, challenge: String },
    HelloAck { node_id: String, node_name: String, echo: String, challenge: String },
    Confirm { echo: String },
    Versions { vector: HashMap<String, i64> },
    Events { events: Vec<SyncEvent> },
    EventsEnd,
    NeedFiles { hashes: Vec<String> },
    FileChunk { hash: String, index: u32, total: u32, data: String },
    FilesEnd,
    Bye,
    Error { message: String },
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    n: u64,
    from_client: bool,
    msg: Msg,
}

pub struct Session<S> {
    ws: WebSocketStream<S>,
    key: [u8; 32],
    is_client: bool,
    send_n: u64,
    recv_n: u64,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Session<S> {
    pub fn new(ws: WebSocketStream<S>, key: [u8; 32], is_client: bool) -> Self {
        Session { ws, key, is_client, send_n: 0, recv_n: 0 }
    }

    pub async fn send(&mut self, msg: Msg) -> Result<(), SyncError> {
        let env = Envelope { n: self.send_n, from_client: self.is_client, msg };
        let plain = serde_json::to_vec(&env)?;
        let (ct, nonce) = crypto::encrypt(&self.key, &plain);
        let mut out = Vec::with_capacity(nonce.len() + ct.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        self.ws.send(Message::Binary(out)).await?;
        self.send_n += 1;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Msg, SyncError> {
        loop {
            let next = tokio::time::timeout(RECV_TIMEOUT, self.ws.next())
                .await
                .map_err(|_| SyncError::Timeout)?;
            match next {
                None => return Err(SyncError::Closed),
                Some(Err(e)) => return Err(e.into()),
                Some(Ok(Message::Binary(b))) => {
                    if b.len() < crypto::NONCE_LEN + 16 {
                        return Err(SyncError::Protocol("رسالة قصيرة جدًا".into()));
                    }
                    let (nonce, ct) = b.split_at(crypto::NONCE_LEN);
                    // فشل فك التشفير = مفتاح مزامنة مختلف أو عبث بالرسالة
                    let plain = crypto::decrypt(&self.key, ct, nonce).map_err(|_| SyncError::Auth)?;
                    let env: Envelope = serde_json::from_slice(&plain)?;
                    if env.from_client == self.is_client {
                        return Err(SyncError::Protocol("رسالة منعكسة".into()));
                    }
                    if env.n != self.recv_n {
                        return Err(SyncError::Protocol("ترتيب الرسائل غير صحيح (إعادة إرسال؟)".into()));
                    }
                    self.recv_n += 1;
                    if let Msg::Error { message } = env.msg {
                        return Err(SyncError::Peer(message));
                    }
                    return Ok(env.msg);
                }
                Some(Ok(Message::Close(_))) => return Err(SyncError::Closed),
                Some(Ok(_)) => continue, // Ping/Pong/Text تُتجاهل
            }
        }
    }
}
