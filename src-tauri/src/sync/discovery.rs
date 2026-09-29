//! اكتشاف الأجهزة على الشبكة المحلية عبر mDNS/Zeroconf (بدون إعداد IP يدوي).

use crate::sync::{PeerInfo, Peers, SyncError};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::{watch, Notify};

pub const SERVICE_TYPE: &str = "_govtarchive._tcp.local.";

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// يعلن عن هذا الجهاز على الشبكة (اسم الخدمة = معرّف الجهاز، فهو فريد).
pub fn advertise(
    daemon: &ServiceDaemon,
    node_id: &str,
    node_name: &str,
    port: u16,
) -> Result<(), SyncError> {
    let host = format!("archive-{}.local.", &node_id[..node_id.len().min(8)]);
    let props = [("node_id", node_id), ("name", node_name)];
    let info = ServiceInfo::new(SERVICE_TYPE, node_id, &host, "", port, &props[..])
        .map_err(|e| SyncError::Protocol(format!("mDNS: {e}")))?
        .enable_addr_auto();
    daemon
        .register(info)
        .map_err(|e| SyncError::Protocol(format!("mDNS: {e}")))
}

pub fn browse(daemon: &ServiceDaemon) -> Result<mdns_sd::Receiver<ServiceEvent>, SyncError> {
    daemon
        .browse(SERVICE_TYPE)
        .map_err(|e| SyncError::Protocol(format!("mDNS: {e}")))
}

/// يراقب أحداث الاكتشاف ويحدّث جدول الأجهزة.
pub async fn watch(
    rx: mdns_sd::Receiver<ServiceEvent>,
    peers: Peers,
    self_id: String,
    discovered: Arc<Notify>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            ev = rx.recv_async() => {
                match ev {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        let Some(node_id) = info.get_property_val_str("node_id").map(|s| s.to_string()) else { continue };
                        if node_id == self_id { continue }
                        let name = info.get_property_val_str("name").unwrap_or("جهاز").to_string();
                        let port = info.get_port();
                        // IPv4 فقط (المستمع مربوط على 0.0.0.0)
                        let addrs: Vec<SocketAddr> = info
                            .get_addresses()
                            .iter()
                            .filter(|ip| matches!(ip, IpAddr::V4(_)))
                            .map(|ip| SocketAddr::new(*ip, port))
                            .collect();
                        if addrs.is_empty() { continue }
                        let mut is_new = false;
                        if let Ok(mut map) = peers.lock() {
                            is_new = !map.contains_key(&node_id);
                            let entry = map.entry(node_id.clone()).or_insert_with(|| PeerInfo {
                                node_id: node_id.clone(),
                                name: name.clone(),
                                addrs: vec![],
                                last_seen: 0,
                                last_sync: None,
                                last_error: None,
                            });
                            entry.name = name;
                            entry.addrs = addrs;
                            entry.last_seen = now();
                        }
                        // جهاز جديد: ابدأ مزامنة فورًا بدل انتظار الجولة الدورية
                        if is_new { discovered.notify_one(); }
                    }
                    Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                        if let Ok(mut map) = peers.lock() {
                            map.retain(|id, _| !fullname.starts_with(id.as_str()));
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            _ = shutdown.changed() => break,
        }
    }
}
