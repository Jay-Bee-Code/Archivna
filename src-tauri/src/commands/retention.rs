use crate::commands::auth::{require_role, SessionState, ROLE_ADMIN};
use crate::commands::documents::{full_document_payload, row_to_document, ArchiveError, Document};
use govt_archive_system::db::DbState;
use govt_archive_system::sync::{store as sync_store, SyncState};
use chrono::Utc;
use rusqlite::params;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
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


/// تعيين مدة الاحتفاظ (بالسنوات) لنوع وثيقة — None يعني "دائم، لا إتلاف تلقائي أبدًا"
#[tauri::command]
pub fn set_document_type_retention(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    document_type_id: String,
    retention_years: Option<i64>,
) -> Result<(), ArchiveError> {
    require_role(&session, &[ROLE_ADMIN])?;
    if let Some(y) = retention_years {
        if y <= 0 {
            return Err(ArchiveError::Invalid("مدة الاحتفاظ يجب أن تكون أكبر من صفر".into()));
        }
    }
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let affected = conn.execute(
        "UPDATE document_types SET retention_years = ?1 WHERE id = ?2",
        params![retention_years, document_type_id],
    )?;
    if affected == 0 {
        return Err(ArchiveError::Invalid("نوع الوثيقة غير موجود".into()));
    }

    let name: String = conn.query_row(
        "SELECT name FROM document_types WHERE id = ?1",
        params![document_type_id], |r| r.get(0),
    )?;
    let created_at: i64 = conn.query_row(
        "SELECT created_at FROM document_types WHERE id = ?1",
        params![document_type_id], |r| r.get(0),
    )?;
    sync_store::log_local_event(
        conn, "document_type", &document_type_id, "upsert",
        &serde_json::json!({ "id": document_type_id, "name": name, "created_at": created_at, "retention_years": retention_years }),
    )?;
    sync.poke();
    Ok(())
}

/// تجميد/رفع تجميد قانوني عن وثيقة — يمنع إتلافها حتى لو انتهت مدة الاحتفاظ
#[tauri::command]
pub fn set_legal_hold(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    id: String,
    hold: bool,
) -> Result<(), ArchiveError> {
    let user = require_role(&session, &[ROLE_ADMIN])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();
    let now = Utc::now().timestamp();

    let affected = conn.execute(
        "UPDATE documents SET legal_hold = ?1, updated_at = ?2 WHERE id = ?3 AND is_deleted = 0",
        params![hold as i64, now, id],
    )?;
    if affected == 0 {
        return Err(ArchiveError::Invalid("الوثيقة غير موجودة".into()));
    }
    conn.execute(
        "INSERT INTO audit_log (id, document_id, user_id, action, timestamp, node_id, metadata)
         VALUES (?1, ?2, ?3, 'legal_hold_changed', ?4, 'local', ?5)",
        params![Uuid::new_v4().to_string(), id, user.id, now, hold.to_string()],
    )?;
    sync_store::log_local_event(conn, "document", &id, "upsert", &full_document_payload(conn, &id)?)?;
    sync.poke();
    Ok(())
}

/// الوثائق المؤهَّلة للإتلاف الآن: نوعها له مدة احتفاظ محدَّدة، انقضت فعليًا،
/// لا تجميد قانوني عليها، ولم تُتلف بعد. للمدير فقط (قرار الإتلاف حسّاس).
#[tauri::command]
pub fn list_disposal_candidates(
    db: State<DbState>,
    session: State<SessionState>,
) -> Result<Vec<Document>, ArchiveError> {
    let user = require_role(&session, &[ROLE_ADMIN])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    // المنطق الفعلي في db/retention.rs (بلا Tauri) ليُختبر مباشرة بوقت ثابت.
    // هنا فقط نجلب الصفوف الكاملة لمعرّفات المؤهَّلة.
    let ids = govt_archive_system::db::retention::disposal_candidate_ids(conn, Utc::now().timestamp())?;
    let cols = "d.id, d.title, d.category_id, d.file_hash, d.file_size, d.mime_type,
                d.created_at, d.updated_at, (d.ocr_text IS NOT NULL AND d.ocr_text <> ''),
                d.department_id, d.document_type_id, d.registry_number, d.confidentiality_level, d.status,
                d.legal_hold, d.disposed_at, d.physical_location, d.physical_status, d.borrowed_by, d.borrowed_at,
                d.correspondent_id";
    let mut docs = Vec::with_capacity(ids.len());
    let mut stmt = conn.prepare(&format!("SELECT {cols} FROM documents d WHERE d.id = ?1"))?;
    for id in ids {
        if let Ok(doc) = stmt.query_row(rusqlite::params![id], row_to_document) {
            docs.push(doc);
        }
    }
    let _ = user; // الدور مُتحقَّق منه أعلاه (المدير فقط)
    Ok(docs)
}

#[derive(Debug, Serialize)]
pub struct DisposalRecord {
    pub id: String,
    pub document_id: String,
    pub title: String,
    pub registry_number: Option<String>,
    pub disposed_by: Option<String>,
    pub disposed_at: i64,
    pub reason: Option<String>,
}

/// الإتلاف الفعلي: يحذف الملف المشفَّر من القرص نهائيًا (إن لم تعد أي وثيقة
/// أخرى حيّة تشير لنفس المحتوى)، ويُبقي محضر إتلاف دائمًا لا يُحذف أبدًا.
/// هذا مختلف جوهريًا عن delete_document (حذف منطقي قابل للتراجع) — لا عودة بعد هذا.
#[tauri::command]
pub fn dispose_document(
    app_handle: AppHandle,
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    id: String,
    reason: Option<String>,
) -> Result<(), ArchiveError> {
    let user = require_role(&session, &[ROLE_ADMIN])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let (title, registry_number, document_type_id, file_hash, legal_hold, disposed_at): (
        String, Option<String>, Option<String>, String, i64, Option<i64>,
    ) = conn
        .query_row(
            "SELECT title, registry_number, document_type_id, file_hash, legal_hold, disposed_at
             FROM documents WHERE id = ?1 AND is_deleted = 0",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .map_err(|_| ArchiveError::Invalid("الوثيقة غير موجودة".into()))?;

    if legal_hold != 0 {
        return Err(ArchiveError::Invalid("الوثيقة مجمَّدة قانونيًا — ارفع التجميد أولًا".into()));
    }
    if disposed_at.is_some() {
        return Err(ArchiveError::Invalid("الوثيقة مُتلَفة بالفعل".into()));
    }

    let now = Utc::now().timestamp();

    // حذف الملف الفعلي فقط إن لم تعد أي وثيقة حيّة أخرى تشير لنفس المحتوى
    // (التخزين Content-Addressed: نفس الملف قد يُستخدَم بأكثر من سجل وثيقة)
    let other_refs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE file_hash = ?1 AND id <> ?2 AND is_deleted = 0",
        params![file_hash, id], |r| r.get(0),
    )?;
    if other_refs == 0 {
        if let Ok(dir) = app_handle.path().app_data_dir() {
            let vault = govt_archive_system::db::vault_dir(&dir);
            if let Ok(path) = sync_store::blob_path(&vault, &file_hash) {
                let _ = std::fs::remove_file(path); // best-effort: قد لا يكون الملف وصل لهذا الجهاز أصلًا
            }
        }
    }

    conn.execute(
        "UPDATE documents SET is_deleted = 1, disposed_at = ?1, status = 'disposed', updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;

    let log_id = Uuid::new_v4().to_string();
    let node = sync_store::node_id(conn)?;
    conn.execute(
        "INSERT INTO disposal_log (id, document_id, title, registry_number, document_type_id, disposed_by, disposed_at, reason, node_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![log_id, id, title, registry_number, document_type_id, user.id, now, reason, node],
    )?;
    conn.execute(
        "INSERT INTO audit_log (id, document_id, user_id, action, timestamp, node_id, metadata)
         VALUES (?1, ?2, ?3, 'disposed', ?4, 'local', ?5)",
        params![Uuid::new_v4().to_string(), id, user.id, now, reason],
    )?;

    // حالة كاملة (وليس حقولًا جزئية فقط) — تحمل disposed_at، فالجهاز المستقبِل
    // يحذف نسخته من الملف أيضًا تلقائيًا (راجع sync/store.rs)
    sync_store::log_local_event(conn, "document", &id, "upsert", &full_document_payload(conn, &id)?)?;
    sync_store::log_local_event(
        conn, "disposal_log", &log_id, "upsert",
        &serde_json::json!({
            "id": log_id, "document_id": id, "title": title, "registry_number": registry_number,
            "document_type_id": document_type_id, "disposed_by": user.id, "disposed_at": now,
            "reason": reason, "node_id": node
        }),
    )?;
    sync.poke();
    Ok(())
}

/// سجل كل عمليات الإتلاف — محضر دائم لا يُحذف، للمدير فقط
#[tauri::command]
pub fn list_disposal_log(db: State<DbState>, session: State<SessionState>) -> Result<Vec<DisposalRecord>, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, document_id, title, registry_number, disposed_by, disposed_at, reason
         FROM disposal_log ORDER BY disposed_at DESC",
    )?;
    let records = stmt
        .query_map([], |r| {
            Ok(DisposalRecord {
                id: r.get(0)?, document_id: r.get(1)?, title: r.get(2)?, registry_number: r.get(3)?,
                disposed_by: r.get(4)?, disposed_at: r.get(5)?, reason: r.get(6)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(records)
}
