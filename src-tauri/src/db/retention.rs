//! منطق الاحتفاظ والإتلاف — بلا Tauri، ليُختبر مباشرة.
//!
//! الوقت `now` معامل صريح (لا يُقرأ داخليًا) فيكون الاختبار حتميًا بتاريخ ثابت.
//! ملاحظة: مقارنة الوقت تُمرَّر كعدد صحيح من Rust. لا نستخدم strftime('%s','now')
//! لأنها تُرجع TEXT في SQLite، ومقارنتها بعدد صحيح تجعل الشرط صحيحًا دائمًا.

use rusqlite::{named_params, Connection};

pub const SECONDS_PER_YEAR: i64 = 365 * 24 * 3600;

/// معرّفات الوثائق المؤهَّلة للإتلاف عند الوقت `now`:
/// نوعها له مدة احتفاظ، انقضت، لا تجميد قانوني، لم تُتلف، ولم تُحذف.
pub fn disposal_candidate_ids(conn: &Connection, now: i64) -> rusqlite::Result<Vec<String>> {
    let sql = format!(
        "SELECT d.id FROM documents d
         JOIN document_types t ON d.document_type_id = t.id
         WHERE t.retention_years IS NOT NULL
           AND d.legal_hold = 0 AND d.disposed_at IS NULL AND d.is_deleted = 0
           AND (d.created_at + t.retention_years * {SECONDS_PER_YEAR}) < :now
         ORDER BY d.created_at ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let ids = stmt
        .query_map(named_params! { ":now": now }, |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}
