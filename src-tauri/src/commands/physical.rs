use crate::commands::auth::{bypasses_visibility_filter, require_role, SessionState, ALL_ROLES, ROLE_ADMIN, ROLE_ARCHIVIST};
use crate::commands::documents::{full_document_payload, ArchiveError};
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

const VALID_PHYSICAL_STATUSES: &[&str] = &["none", "present", "borrowed", "missing", "destroyed"];

/// تحديث موقع وحالة الأصل الورقي معًا — عند status="borrowed" يُسجَّل المستعير
/// تلقائيًا كالمستخدم الحالي؛ أي حالة أخرى تمسح بيانات الإعارة
#[tauri::command]
pub fn update_physical(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    id: String,
    location: Option<String>,
    status: String,
) -> Result<(), ArchiveError> {
    let user = require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST])?;
    if !VALID_PHYSICAL_STATUSES.contains(&status.as_str()) {
        return Err(ArchiveError::Invalid(format!("حالة أصل ورقي غير معروفة: {status}")));
    }

    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();
    let now = Utc::now().timestamp();

    let (borrowed_by, borrowed_at): (Option<String>, Option<i64>) = if status == "borrowed" {
        (Some(user.id.clone()), Some(now))
    } else {
        (None, None)
    };

    let affected = conn.execute(
        "UPDATE documents SET physical_location = ?1, physical_status = ?2,
            borrowed_by = ?3, borrowed_at = ?4, updated_at = ?5
         WHERE id = ?6 AND is_deleted = 0",
        params![location, status, borrowed_by, borrowed_at, now, id],
    )?;
    if affected == 0 {
        return Err(ArchiveError::Invalid("الوثيقة غير موجودة".into()));
    }

    conn.execute(
        "INSERT INTO audit_log (id, document_id, user_id, action, timestamp, node_id, metadata)
         VALUES (?1, ?2, ?3, 'physical_status_changed', ?4, 'local', ?5)",
        params![Uuid::new_v4().to_string(), id, user.id, now, status],
    )?;
    sync_store::log_local_event(conn, "document", &id, "upsert", &full_document_payload(conn, &id)?)?;
    sync.poke();
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct QrResult {
    pub svg: String,
    pub payload: String,
}

/// يولّد رمز QR لطباعته على غلاف الملف الورقي — يحمل رقم القيد إن وُجد
/// (مقروء بشريًا لأي ماسح عام)، وإلا معرّف الوثيقة كبديل
#[tauri::command]
pub fn get_document_qr(
    db: State<DbState>,
    session: State<SessionState>,
    id: String,
) -> Result<QrResult, ArchiveError> {
    let user = require_role(&session, ALL_ROLES)?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let (registry_number, conf, dept): (Option<String>, i64, Option<String>) = conn
        .query_row(
            "SELECT registry_number, confidentiality_level, department_id FROM documents WHERE id = ?1 AND is_deleted = 0",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| ArchiveError::Invalid("الوثيقة غير موجودة".into()))?;

    if !bypasses_visibility_filter(&user.role) {
        let dept_ok = user.department_id.is_none() || dept.is_none() || dept == user.department_id;
        if conf > user.clearance_level || !dept_ok {
            return Err(ArchiveError::Invalid("لا تملك صلاحية اطّلاع كافية لهذه الوثيقة".into()));
        }
    }

    let payload = registry_number.unwrap_or(id);
    let svg = crate::qr::generate_svg(&payload).map_err(ArchiveError::Invalid)?;
    Ok(QrResult { svg, payload })
}
