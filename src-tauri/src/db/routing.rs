//! منطق صندوق الإحالات الوارد — بلا Tauri، ليُختبر مباشرة.

use rusqlite::{params, Connection};

/// إحالة واردة للمستخدم: معرّف الإحالة، الوثيقة، عنوانها، المُحيل، ووقت الإحالة، والملاحظة.
#[derive(Debug, Clone, PartialEq)]
pub struct InboxRow {
    pub id: String,
    pub document_id: String,
    pub document_title: String,
    pub routed_by: Option<String>,
    pub routed_by_username: Option<String>,
    pub routed_at: i64,
    pub note: Option<String>,
}

/// إحالات المستخدم `user_id`: تُستبعد الوثائق المحذوفة منطقيًا، والأحدث أولًا، بحد 100.
pub fn inbox_for(conn: &Connection, user_id: &str) -> rusqlite::Result<Vec<InboxRow>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.document_id, d.title, r.routed_by, u.username, r.routed_at, r.note
         FROM document_routing r
         JOIN documents d ON r.document_id = d.id
         LEFT JOIN users u ON r.routed_by = u.id
         WHERE r.routed_to_user_id = ?1 AND d.is_deleted = 0
         ORDER BY r.routed_at DESC
         LIMIT 100",
    )?;
    let rows = stmt
        .query_map(params![user_id], |r| {
            Ok(InboxRow {
                id: r.get(0)?,
                document_id: r.get(1)?,
                document_title: r.get(2)?,
                routed_by: r.get(3)?,
                routed_by_username: r.get(4)?,
                routed_at: r.get(5)?,
                note: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
