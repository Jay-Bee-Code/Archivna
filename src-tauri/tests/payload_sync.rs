//! اختبار تكامل: الحمولة الحقيقية لـ full_document_payload تمر عبر apply_remote_events
//! على جهاز ثانٍ، ثم يجب أن يطابق الصف الأصل في كل عمود منقول.
//!
//! هذا يلتقط الخلل الذي كان سيكسر المزامنة: أي عدم تطابق بين مفاتيح الحمولة وجملة
//! INSERT في sync/store.rs. وهو اختبار على الدالة الفعلية، لا نسخة معاد كتابتها.

use govt_archive_system::crypto;
use govt_archive_system::db;
use govt_archive_system::db::payload::full_document_payload;
use govt_archive_system::sync::store::{self, SyncEvent};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn fresh() -> (Connection, PathBuf) {
    let conn = Connection::open_in_memory().unwrap();
    db::apply_migrations(&conn).unwrap();
    let dir = std::env::temp_dir().join(format!("payload-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    (conn, dir)
}

/// الصفوف الأم أولًا (المفاتيح الأجنبية مفعّلة)، ثم وثيقة بكل حقولها غير الافتراضية
fn seed_full_document(conn: &Connection, id: &str) -> String {
    conn.execute_batch(
        "INSERT INTO categories (id, name, created_at) VALUES ('cat-1','فئة',1);
         INSERT INTO departments (id, name_ar, created_at) VALUES ('dept-1','قسم',1);
         INSERT INTO document_types (id, name, created_at) VALUES ('type-1','نوع',1);
         INSERT INTO correspondents (id, name, kind, created_at) VALUES ('corr-1','جهة','ministry',1);
         INSERT INTO users (id, username, password_hash, role, created_at) VALUES ('user-1','u1','h','archivist',1);
         INSERT INTO users (id, username, password_hash, role, created_at) VALUES ('user-9','u9','h','archivist',1);",
    )
    .unwrap();
    let hash = format!("{:x}", Sha256::digest(b"payload-test"));
    conn.execute(
        "INSERT INTO documents
            (id, title, category_id, file_hash, file_path, file_size, mime_type, ocr_text,
             department_id, document_type_id, registry_number, confidentiality_level, status, metadata,
             legal_hold, disposed_at, physical_location, physical_status, borrowed_by, borrowed_at,
             correspondent_id, created_by, created_at, updated_at, origin_node_id, is_deleted)
         VALUES (?1,'عنوان كامل','cat-1',?2,'/x',4096,'application/pdf','نص OCR',
                 'dept-1','type-1','2026/DRH/0007-ABC123',3,'in_review','{\"k\":1}',
                 1,NULL,'مبنى أ / رف 3','borrowed','user-9',555,
                 'corr-1','user-1',1000,2000,'node-A',0)",
        params![id, hash],
    )
    .unwrap();
    hash
}

#[test]
fn full_payload_executes_and_has_every_expected_field() {
    let (conn, _) = fresh();
    let hash = seed_full_document(&conn, "doc-1");
    let p = full_document_payload(&conn, "doc-1").expect("الدالة الحقيقية يجب أن تُنفَّذ");

    assert_eq!(p["title"], "عنوان كامل");
    assert_eq!(p["file_hash"], hash.as_str());
    assert_eq!(p["confidentiality_level"], 3);
    assert_eq!(p["legal_hold"], 1);
    assert!(p["disposed_at"].is_null());
    assert_eq!(p["physical_status"], "borrowed");
    assert_eq!(p["borrowed_by"], "user-9");
    assert_eq!(p["correspondent_id"], "corr-1");
    assert_eq!(p["origin_node_id"], "node-A");
    assert_eq!(p["is_deleted"], 0);
}

#[test]
fn missing_document_is_an_error_not_a_panic() {
    let (conn, _) = fresh();
    assert!(full_document_payload(&conn, "nope").is_err());
}

#[test]
fn payload_round_trips_through_sync_to_an_identical_row() {
    let (a, _) = fresh();
    let (b, b_dir) = fresh();
    seed_full_document(&a, "doc-1");
    let payload = full_document_payload(&a, "doc-1").unwrap();

    let ev = SyncEvent {
        origin_node_id: "node-A".into(),
        origin_seq: 1,
        entity_type: "document".into(),
        entity_id: "doc-1".into(),
        operation: "upsert".into(),
        payload,
        lamport: 1,
        created_at: 1,
    };
    assert_eq!(store::apply_remote_events(&b, &[ev], &b_dir).unwrap(), 1);

    // الأعمدة المنقولة كلها ما عدا file_path (مسار محلي لكل جهاز)
    let cols = "title, category_id, file_hash, file_size, mime_type, ocr_text, department_id,
                document_type_id, registry_number, confidentiality_level, status, metadata,
                legal_hold, disposed_at, physical_location, physical_status, borrowed_by,
                borrowed_at, correspondent_id, created_by, created_at, updated_at, origin_node_id, is_deleted";
    let n = cols.split(',').count();
    let dump = |c: &Connection| -> Vec<String> {
        c.query_row(&format!("SELECT {cols} FROM documents WHERE id='doc-1'"), [], |r| {
            (0..n)
                .map(|i| Ok(format!("{:?}", r.get::<_, rusqlite::types::Value>(i)?)))
                .collect::<Result<Vec<_>, rusqlite::Error>>()
        })
        .unwrap()
    };
    let (da, db_) = (dump(&a), dump(&b));
    assert_eq!(da.len(), n);
    assert_eq!(da, db_, "الصف على B يجب أن يطابق A في كل عمود منقول");
}
