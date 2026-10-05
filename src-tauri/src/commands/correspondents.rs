use crate::commands::auth::{require_role, SessionState, ALL_ROLES, ROLE_ADMIN, ROLE_ARCHIVIST};
use crate::commands::documents::ArchiveError;
use crate::db::DbState;
use crate::sync::{store as sync_store, SyncState};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

const VALID_KINDS: &[&str] = &["ministry", "wilaya", "company", "individual", "other"];

#[derive(Debug, Serialize, Deserialize)]
pub struct Correspondent {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct NewCorrespondentInput {
    pub name: String,
    pub kind: String,
    pub address: Option<String>,
}

macro_rules! require_conn {
    ($db:expr) => {{
        let guard = $db.0.lock().unwrap();
        match guard.as_ref() {
            Some(_) => guard,
            None => return Err(ArchiveError::Invalid("الخزنة مقفلة — افتحها أولًا".into())),
        }
    }};
}

#[tauri::command]
pub fn add_correspondent(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    input: NewCorrespondentInput,
) -> Result<Correspondent, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST])?;
    if input.name.trim().is_empty() {
        return Err(ArchiveError::Invalid("اسم الجهة مطلوب".into()));
    }
    let kind = if VALID_KINDS.contains(&input.kind.as_str()) { input.kind } else { "other".to_string() };

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    conn.execute(
        "INSERT INTO correspondents (id, name, kind, address, is_active, created_at) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
        params![id, input.name, kind, input.address, now],
    )?;
    sync_store::log_local_event(
        conn, "correspondent", &id, "upsert",
        &serde_json::json!({
            "id": id, "name": input.name, "kind": kind, "address": input.address,
            "is_active": 1, "created_at": now
        }),
    )?;
    sync.poke();

    Ok(Correspondent { id, name: input.name, kind, address: input.address, is_active: true, created_at: now })
}

#[tauri::command]
pub fn list_correspondents(db: State<DbState>, session: State<SessionState>) -> Result<Vec<Correspondent>, ArchiveError> {
    require_role(&session, ALL_ROLES)?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, name, kind, address, is_active, created_at FROM correspondents WHERE is_active = 1 ORDER BY name ASC",
    )?;
    let items = stmt
        .query_map([], |r| {
            Ok(Correspondent {
                id: r.get(0)?, name: r.get(1)?, kind: r.get(2)?, address: r.get(3)?,
                is_active: r.get(4)?, created_at: r.get(5)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(items)
}

#[tauri::command]
pub fn delete_correspondent(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    id: String,
) -> Result<(), ArchiveError> {
    require_role(&session, &[ROLE_ADMIN])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let doc_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE correspondent_id = ?1 AND is_deleted = 0",
        params![id], |r| r.get(0),
    )?;
    if doc_count > 0 {
        return Err(ArchiveError::Invalid("لا يمكن حذف جهة مرتبطة بوثائق — عدّل الوثائق أولًا".into()));
    }

    conn.execute("UPDATE correspondents SET is_active = 0 WHERE id = ?1", params![id])?;
    sync_store::log_local_event(conn, "correspondent", &id, "delete", &serde_json::json!({ "id": id }))?;
    sync.poke();
    Ok(())
}
