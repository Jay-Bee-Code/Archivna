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
pub struct Category {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: i64,
    pub document_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct NewCategoryInput {
    pub name: String,
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

/// إنشاء فئة جديدة — مسموح فقط لـ admin و archivist
#[tauri::command]
pub fn add_category(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    input: NewCategoryInput,
) -> Result<Category, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST])?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();

    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, parent_id, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, input.name, input.parent_id, now],
    )?;
    sync_store::log_local_event(
        conn,
        "category",
        &id,
        "upsert",
        &serde_json::json!({ "id": id, "name": input.name, "parent_id": input.parent_id, "created_at": now }),
    )?;
    sync.poke();

    Ok(Category { id, name: input.name, parent_id: input.parent_id, created_at: now, document_count: 0 })
}

/// جلب كل الفئات — لأي مستخدم مسجَّل دخوله
#[tauri::command]
pub fn list_categories(
    db: State<DbState>,
    session: State<SessionState>,
) -> Result<Vec<Category>, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.parent_id, c.created_at,
                (SELECT COUNT(*) FROM documents d WHERE d.category_id = c.id AND d.is_deleted = 0)
         FROM categories c ORDER BY c.name ASC",
    )?;
    let categories = stmt
        .query_map([], |row| {
            Ok(Category {
                id: row.get(0)?, name: row.get(1)?, parent_id: row.get(2)?,
                created_at: row.get(3)?, document_count: row.get(4)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(categories)
}

/// حذف فئة — مسموح فقط لـ admin؛ يُرفض إن كانت تحتوي وثائق أو فئات فرعية
#[tauri::command]
pub fn delete_category(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    id: String,
) -> Result<(), ArchiveError> {
    require_role(&session, &[ROLE_ADMIN])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let doc_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE category_id = ?1 AND is_deleted = 0",
        params![id], |row| row.get(0),
    )?;
    if doc_count > 0 {
        return Err(ArchiveError::Invalid("لا يمكن حذف فئة تحتوي وثائق — انقل الوثائق أولًا".into()));
    }
    let child_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE parent_id = ?1",
        params![id], |row| row.get(0),
    )?;
    if child_count > 0 {
        return Err(ArchiveError::Invalid("لا يمكن حذف فئة تحتوي فئات فرعية".into()));
    }

    conn.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
    sync_store::log_local_event(conn, "category", &id, "delete", &serde_json::json!({ "id": id }))?;
    sync.poke();
    Ok(())
}

/// جلب وثائق فئة معيّنة (للفلترة)، مقيَّدة بنفس قواعد رؤية القسم والاطّلاع
#[tauri::command]
pub fn list_documents_by_category(
    db: State<DbState>,
    session: State<SessionState>,
    category_id: String,
) -> Result<Vec<crate::commands::documents::Document>, ArchiveError> {
    use crate::commands::auth::bypasses_visibility_filter;
    use crate::commands::documents::{row_to_document, visibility_sql, DOC_COLUMNS};
    let user = require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER])?;

    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();
    let sql = format!(
        "SELECT {DOC_COLUMNS} FROM documents WHERE category_id = :cat AND is_deleted = 0{} ORDER BY created_at DESC",
        visibility_sql(&user.role)
    );
    let mut stmt = conn.prepare(&sql)?;
    let docs = if bypasses_visibility_filter(&user.role) {
        stmt.query_map(rusqlite::named_params! { ":cat": category_id }, row_to_document)?
            .filter_map(Result::ok)
            .collect()
    } else {
        stmt.query_map(
            rusqlite::named_params! { ":cat": category_id, ":A": user.clearance_level, ":B": user.department_id },
            row_to_document,
        )?
        .filter_map(Result::ok)
        .collect()
    };
    Ok(docs)
}
