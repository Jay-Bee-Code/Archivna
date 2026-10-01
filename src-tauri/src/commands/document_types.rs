use crate::commands::auth::{require_role, SessionState, ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER};
use crate::commands::documents::ArchiveError;
use crate::db::DbState;
use crate::sync::{store as sync_store, SyncState};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentType {
    pub id: String,
    pub name: String,
    pub created_at: i64,
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

/// قائمة أنواع مقترحة للإعداد الأول — الواجهة تعرضها كأزرار سريعة، ولا تُنشأ تلقائيًا
pub const SUGGESTED_TYPES: &[&str] = &[
    "مراسلة واردة", "مراسلة صادرة", "قرار", "مقرر", "مرسوم", "تعليمة", "منشور",
    "عقد / صفقة", "محضر اجتماع", "تقرير", "ملف موظف", "وثيقة مالية", "شهادة / رخصة",
];

/// قائمة الأنواع المقترحة — تُعرض كأزرار اختيار سريع عند أول إعداد لأنواع الوثائق
#[tauri::command]
pub fn suggested_document_types() -> Vec<&'static str> {
    SUGGESTED_TYPES.to_vec()
}

#[tauri::command]
pub fn add_document_type(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    name: String,
) -> Result<DocumentType, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST])?;
    if name.trim().is_empty() {
        return Err(ArchiveError::Invalid("اسم النوع مطلوب".into()));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    conn.execute(
        "INSERT INTO document_types (id, name, created_at) VALUES (?1, ?2, ?3)",
        params![id, name, now],
    )?;
    sync_store::log_local_event(
        conn, "document_type", &id, "upsert",
        &serde_json::json!({ "id": id, "name": name, "created_at": now }),
    )?;
    sync.poke();

    Ok(DocumentType { id, name, created_at: now })
}

#[tauri::command]
pub fn list_document_types(
    db: State<DbState>,
    session: State<SessionState>,
) -> Result<Vec<DocumentType>, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let mut stmt = conn.prepare("SELECT id, name, created_at FROM document_types ORDER BY name ASC")?;
    let types = stmt
        .query_map([], |row| Ok(DocumentType { id: row.get(0)?, name: row.get(1)?, created_at: row.get(2)? }))?
        .filter_map(Result::ok)
        .collect();
    Ok(types)
}

#[tauri::command]
pub fn delete_document_type(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    id: String,
) -> Result<(), ArchiveError> {
    require_role(&session, &[ROLE_ADMIN])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let doc_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE document_type_id = ?1 AND is_deleted = 0",
        params![id], |r| r.get(0),
    )?;
    if doc_count > 0 {
        return Err(ArchiveError::Invalid("لا يمكن حذف نوع مستخدَم في وثائق قائمة".into()));
    }

    conn.execute("DELETE FROM document_types WHERE id = ?1", params![id])?;
    sync_store::log_local_event(conn, "document_type", &id, "delete", &serde_json::json!({ "id": id }))?;
    sync.poke();
    Ok(())
}
