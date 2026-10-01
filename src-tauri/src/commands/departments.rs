use crate::commands::auth::{require_role, SessionState, ROLE_ADMIN};
use crate::commands::documents::ArchiveError;
use crate::db::DbState;
use crate::sync::{store as sync_store, SyncState};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Department {
    pub id: String,
    pub name_ar: String,
    pub name_fr: Option<String>,
    pub code: Option<String>,
    pub parent_id: Option<String>,
    pub head_user_id: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct NewDepartmentInput {
    pub name_ar: String,
    pub name_fr: Option<String>,
    pub code: Option<String>,
    pub parent_id: Option<String>,
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

/// إنشاء قسم جديد — للمدير فقط (الهيكل التنظيمي قرار إداري مركزي)
#[tauri::command]
pub fn add_department(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    input: NewDepartmentInput,
) -> Result<Department, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN])?;
    if input.name_ar.trim().is_empty() {
        return Err(ArchiveError::Invalid("اسم القسم مطلوب".into()));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    conn.execute(
        "INSERT INTO departments (id, name_ar, name_fr, code, parent_id, is_active, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        params![id, input.name_ar, input.name_fr, input.code, input.parent_id, now],
    )?;
    sync_store::log_local_event(
        conn,
        "department",
        &id,
        "upsert",
        &serde_json::json!({
            "id": id, "name_ar": input.name_ar, "name_fr": input.name_fr, "code": input.code,
            "parent_id": input.parent_id, "head_user_id": null, "is_active": 1, "created_at": now
        }),
    )?;
    sync.poke();

    Ok(Department {
        id,
        name_ar: input.name_ar,
        name_fr: input.name_fr,
        code: input.code,
        parent_id: input.parent_id,
        head_user_id: None,
        is_active: true,
        created_at: now,
    })
}

/// جلب الأقسام النشطة — لأي مستخدم مسجَّل دخوله (يحتاجها اختيار القسم عند الرفع)
#[tauri::command]
pub fn list_departments(
    db: State<DbState>,
    session: State<SessionState>,
) -> Result<Vec<Department>, ArchiveError> {
    require_role(
        &session,
        &[
            crate::commands::auth::ROLE_ADMIN,
            crate::commands::auth::ROLE_ARCHIVIST,
            crate::commands::auth::ROLE_REVIEWER,
            crate::commands::auth::ROLE_VIEWER,
        ],
    )?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, name_ar, name_fr, code, parent_id, head_user_id, is_active, created_at
         FROM departments WHERE is_active = 1 ORDER BY name_ar ASC",
    )?;
    let deps = stmt
        .query_map([], |row| {
            Ok(Department {
                id: row.get(0)?, name_ar: row.get(1)?, name_fr: row.get(2)?, code: row.get(3)?,
                parent_id: row.get(4)?, head_user_id: row.get(5)?, is_active: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(deps)
}

/// تعطيل قسم (Soft Delete) — يُرفض إن كان له مستخدمون أو وثائق أو أقسام فرعية نشطة
#[tauri::command]
pub fn delete_department(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    id: String,
) -> Result<(), ArchiveError> {
    require_role(&session, &[ROLE_ADMIN])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let user_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE department_id = ?1 AND is_active = 1",
        params![id], |r| r.get(0),
    )?;
    if user_count > 0 {
        return Err(ArchiveError::Invalid("لا يمكن تعطيل قسم به مستخدمون نشطون".into()));
    }
    let doc_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE department_id = ?1 AND is_deleted = 0",
        params![id], |r| r.get(0),
    )?;
    if doc_count > 0 {
        return Err(ArchiveError::Invalid("لا يمكن تعطيل قسم به وثائق".into()));
    }
    let child_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM departments WHERE parent_id = ?1 AND is_active = 1",
        params![id], |r| r.get(0),
    )?;
    if child_count > 0 {
        return Err(ArchiveError::Invalid("لا يمكن تعطيل قسم به أقسام فرعية نشطة".into()));
    }

    conn.execute("UPDATE departments SET is_active = 0 WHERE id = ?1", params![id])?;
    sync_store::log_local_event(conn, "department", &id, "delete", &serde_json::json!({ "id": id }))?;
    sync.poke();
    Ok(())
}
