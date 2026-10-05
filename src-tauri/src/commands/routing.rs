use crate::commands::auth::{bypasses_visibility_filter, require_role, SessionState, ALL_ROLES, ROLE_ADMIN, ROLE_ARCHIVIST};
use crate::commands::documents::ArchiveError;
use crate::db::DbState;
use crate::sync::{store as sync_store, SyncState};
use chrono::Utc;
use rusqlite::params;
use serde::Serialize;
use tauri::State;
use uuid::Uuid;

macro_rules! require_conn {
    ($db:expr) => {{
        let guard = $db.0.lock().unwrap();
        match guard.as_ref() {
            Some(_) => guard,
            None => return Err(ArchiveError::Invalid("الخزنة مقفلة — افتحها أولًا".into())),
        }
    }};
}

#[derive(Debug, Serialize)]
pub struct RoutingEntry {
    pub id: String,
    pub document_id: String,
    pub routed_to_user_id: String,
    pub routed_to_username: Option<String>,
    pub routed_by: Option<String>,
    pub routed_at: i64,
    pub note: Option<String>,
}

/// يُحيل نسخة من وثيقة لمستخدم آخر داخل المؤسسة — سجل تراكمي لا يُعدَّل ولا يُحذف
#[tauri::command]
pub fn route_document(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    document_id: String,
    to_user_id: String,
    note: Option<String>,
) -> Result<(), ArchiveError> {
    let user = require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    // نفس قواعد رؤية الوثيقة نفسها — لا تُحال وثيقة لا تملك صلاحية رؤيتها أصلًا
    let (conf, dept): (i64, Option<String>) = conn
        .query_row(
            "SELECT confidentiality_level, department_id FROM documents WHERE id = ?1 AND is_deleted = 0",
            params![document_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| ArchiveError::Invalid("الوثيقة غير موجودة".into()))?;
    if !bypasses_visibility_filter(&user.role) {
        let dept_ok = user.department_id.is_none() || dept.is_none() || dept == user.department_id;
        if conf > user.clearance_level || !dept_ok {
            return Err(ArchiveError::Invalid("لا تملك صلاحية اطّلاع كافية لإحالة هذه الوثيقة".into()));
        }
    }

    let target_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE id = ?1 AND is_active = 1",
        params![to_user_id], |r| r.get(0),
    )?;
    if target_exists == 0 {
        return Err(ArchiveError::Invalid("المستخدم المُحال إليه غير موجود أو معطَّل".into()));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO document_routing (id, document_id, routed_to_user_id, routed_by, routed_at, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, document_id, to_user_id, user.id, now, note],
    )?;
    conn.execute(
        "INSERT INTO audit_log (id, document_id, user_id, action, timestamp, node_id, metadata)
         VALUES (?1, ?2, ?3, 'routed', ?4, 'local', ?5)",
        params![Uuid::new_v4().to_string(), document_id, user.id, now, to_user_id],
    )?;
    sync_store::log_local_event(
        conn, "document_routing", &id, "upsert",
        &serde_json::json!({
            "id": id, "document_id": document_id, "routed_to_user_id": to_user_id,
            "routed_by": user.id, "routed_at": now, "note": note
        }),
    )?;
    sync.poke();
    Ok(())
}

/// سجل إحالات وثيقة معيّنة — لأي مستخدم يملك صلاحية رؤية الوثيقة نفسها
#[tauri::command]
pub fn list_document_routing(
    db: State<DbState>,
    session: State<SessionState>,
    document_id: String,
) -> Result<Vec<RoutingEntry>, ArchiveError> {
    require_role(&session, ALL_ROLES)?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let mut stmt = conn.prepare(
        "SELECT r.id, r.document_id, r.routed_to_user_id, u.username, r.routed_by, r.routed_at, r.note
         FROM document_routing r LEFT JOIN users u ON r.routed_to_user_id = u.id
         WHERE r.document_id = ?1 ORDER BY r.routed_at DESC",
    )?;
    let items = stmt
        .query_map(params![document_id], |r| {
            Ok(RoutingEntry {
                id: r.get(0)?, document_id: r.get(1)?, routed_to_user_id: r.get(2)?,
                routed_to_username: r.get(3)?, routed_by: r.get(4)?, routed_at: r.get(5)?, note: r.get(6)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(items)
}
