use crate::commands::auth::{
    require_role, SessionState, ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER,
};
use crate::commands::documents::ArchiveError;
use crate::crypto;
use crate::db::{self, DbState, VaultKeyState};
use crate::sync::net::SyncCore;
use crate::sync::{self, store, PeerView, SyncState};
use rusqlite::Connection;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};

type SharedDb = Arc<Mutex<Option<Connection>>>;
type SharedKey = Arc<Mutex<Option<[u8; 32]>>>;

/// يشغّل محرك المزامنة (ويوقف أي محرك سابق). يُستدعى عند فتح الخزنة إن كان
/// مفتاح المزامنة محفوظًا، وعند تعيين مفتاح جديد.
pub fn start_engine(
    app: &AppHandle,
    db: SharedDb,
    vault_key: SharedKey,
    sync_state: &SyncState,
    sync_key: [u8; 32],
) -> Result<(), ArchiveError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| ArchiveError::Io(e.to_string()))?;
    let vault_dir = db::vault_dir(&dir);

    let (node_id, node_name) = {
        let g = db.lock().unwrap();
        let conn = g
            .as_ref()
            .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;
        store::ensure_node_config(conn)?
    };

    let old = sync_state.0.lock().unwrap().take();
    if let Some(old) = old {
        old.stop();
    }

    let core = SyncCore {
        db,
        vault_key,
        vault_dir,
        sync_key,
        node_id,
        node_name,
        session_lock: Arc::new(tokio::sync::Mutex::new(())),
    };
    let (engine, runner) = sync::prepare(core)?;
    tauri::async_runtime::spawn(runner.run());
    *sync_state.0.lock().unwrap() = Some(engine);
    Ok(())
}

/// عند فتح الخزنة: إن كان مفتاح المزامنة محفوظًا، شغّل المحرك تلقائيًا.
pub fn start_engine_if_configured(
    app: &AppHandle,
    db: SharedDb,
    vault_key: SharedKey,
    sync_state: &SyncState,
) {
    let key = {
        let vk = vault_key.lock().unwrap();
        let dg = db.lock().unwrap();
        match (vk.as_ref(), dg.as_ref()) {
            (Some(k), Some(c)) => store::load_sync_key(c, k).ok().flatten(),
            _ => None,
        }
    };
    if let Some(k) = key {
        let _ = start_engine(app, db, vault_key, sync_state, k);
    }
}

#[derive(Serialize)]
pub struct SyncStatusResponse {
    pub configured: bool,
    pub running: bool,
    pub node_id: Option<String>,
    pub node_name: Option<String>,
    pub peers: Vec<PeerView>,
    pub last_sync: Option<i64>,
}

/// حالة المزامنة — لا تتطلب تسجيل دخول (تُستخدم أيضًا في شاشة الانضمام لشبكة).
#[tauri::command]
pub fn sync_status(
    db: State<DbState>,
    vault_key: State<VaultKeyState>,
    sync: State<SyncState>,
) -> Result<SyncStatusResponse, ArchiveError> {
    let configured = {
        let vk = vault_key.0.lock().unwrap();
        let dg = db.0.lock().unwrap();
        match (vk.as_ref(), dg.as_ref()) {
            (Some(k), Some(c)) => store::load_sync_key(c, k).ok().flatten().is_some(),
            _ => false,
        }
    };
    let status = sync.0.lock().unwrap().as_ref().map(|e| e.status());
    Ok(match status {
        Some(s) => SyncStatusResponse {
            configured,
            running: true,
            node_id: Some(s.node_id),
            node_name: Some(s.node_name),
            peers: s.peers,
            last_sync: s.last_sync,
        },
        None => SyncStatusResponse {
            configured,
            running: false,
            node_id: None,
            node_name: None,
            peers: vec![],
            last_sync: None,
        },
    })
}

/// هل يوجد أي مستخدم في هذه الخزنة؟ (شاشة الانضمام تنتظر وصول الحسابات عبر المزامنة)
#[tauri::command]
pub fn has_users(db: State<DbState>) -> Result<bool, ArchiveError> {
    let g = db.0.lock().unwrap();
    let conn = g
        .as_ref()
        .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
    Ok(n > 0)
}

/// تعيين مفتاح مزامنة المؤسسة وتشغيل المحرك.
/// مسموح: (1) قبل وجود أي مستخدم (إعداد أول/انضمام لشبكة)، أو (2) للمدير فقط بعد ذلك.
#[tauri::command]
pub fn set_sync_key(
    app_handle: AppHandle,
    db: State<DbState>,
    vault_key: State<VaultKeyState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    passphrase: String,
) -> Result<(), ArchiveError> {
    if passphrase.len() < 8 {
        return Err(ArchiveError::Invalid(
            "مفتاح المزامنة يجب أن يكون 8 أحرف على الأقل".into(),
        ));
    }

    let has_users = {
        let g = db.0.lock().unwrap();
        let conn = g
            .as_ref()
            .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
        n > 0
    };
    if has_users {
        require_role(&session, &[ROLE_ADMIN])?;
    }

    let key = crypto::derive_sync_key(&passphrase);
    let vk = vault_key
        .0
        .lock()
        .unwrap()
        .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;
    {
        let g = db.0.lock().unwrap();
        let conn = g
            .as_ref()
            .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;
        store::save_sync_key(conn, &vk, &key)?;
    }

    start_engine(&app_handle, db.0.clone(), vault_key.0.clone(), &sync, key)
}

/// مزامنة فورية مع كل الأجهزة المكتشفة.
#[tauri::command]
pub fn sync_now(session: State<SessionState>, sync: State<SyncState>) -> Result<(), ArchiveError> {
    require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER])?;
    if let Some(e) = sync.0.lock().unwrap().as_ref() {
        e.trigger_now();
    }
    Ok(())
}
