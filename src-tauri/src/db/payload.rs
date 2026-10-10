//! بناء حمولة المزامنة الكاملة للوثيقة — في وحدة بلا أي اعتماد على Tauri عمدًا،
//! حتى يستدعي الاختبار *الدالة الحقيقية نفسها* بدل نسخة منها.
//!
//! خلفية: كانت هذه الدالة في commands/documents.rs (حبيسة اعتماد Tauri)، فبنت
//! الاختبارات الحمولة يدويًا ولم تُنفِّذها قط — رغم أن كل أمر يغيّر وثيقة
//! (حالة، تجميد، أرشيف ورقي، إتلاف) يعتمد عليها. نفس نمط خطأ
//! list_disposal_candidates: اختبار لمنطق مُعاد كتابته لا للكود الفعلي.

use rusqlite::{params, Connection};

/// حالة الوثيقة كاملة كـ JSON. **إلزامي** لأي حدث "document"/"upsert":
/// sync/store.rs يتطلب الحالة كاملة في كل حدث upsert (وليس تغييرًا جزئيًا)،
/// لأن الحدث الفائز بترتيب Last-Writer-Wins قد لا يكون آخر حدث وصل فعليًا.
pub fn full_document_payload(conn: &Connection, id: &str) -> rusqlite::Result<serde_json::Value> {
    conn.query_row(
        "SELECT title, category_id, file_hash, file_size, mime_type, ocr_text,
                department_id, document_type_id, registry_number, confidentiality_level, status, metadata,
                legal_hold, disposed_at, physical_location, physical_status, borrowed_by, borrowed_at,
                created_by, created_at, updated_at, origin_node_id, is_deleted, correspondent_id
         FROM documents WHERE id = ?1",
        params![id],
        |r| {
            Ok(serde_json::json!({
                "id": id,
                "title": r.get::<_, String>(0)?,
                "category_id": r.get::<_, Option<String>>(1)?,
                "file_hash": r.get::<_, String>(2)?,
                "file_size": r.get::<_, i64>(3)?,
                "mime_type": r.get::<_, Option<String>>(4)?,
                "ocr_text": r.get::<_, Option<String>>(5)?,
                "department_id": r.get::<_, Option<String>>(6)?,
                "document_type_id": r.get::<_, Option<String>>(7)?,
                "registry_number": r.get::<_, Option<String>>(8)?,
                "confidentiality_level": r.get::<_, i64>(9)?,
                "status": r.get::<_, String>(10)?,
                "metadata": r.get::<_, Option<String>>(11)?,
                "legal_hold": r.get::<_, i64>(12)?,
                "disposed_at": r.get::<_, Option<i64>>(13)?,
                "physical_location": r.get::<_, Option<String>>(14)?,
                "physical_status": r.get::<_, String>(15)?,
                "borrowed_by": r.get::<_, Option<String>>(16)?,
                "borrowed_at": r.get::<_, Option<i64>>(17)?,
                "created_by": r.get::<_, Option<String>>(18)?,
                "created_at": r.get::<_, i64>(19)?,
                "updated_at": r.get::<_, Option<i64>>(20)?,
                "origin_node_id": r.get::<_, String>(21)?,
                "is_deleted": r.get::<_, i64>(22)?,
                "correspondent_id": r.get::<_, Option<String>>(23)?,
            }))
        },
    )
}
