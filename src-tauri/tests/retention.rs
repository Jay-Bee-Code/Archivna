//! اختبار تكامل لمنطق الإتلاف الحقيقي (db::retention::disposal_candidate_ids).
//!
//! يتحقق من الخلل الذي كشف عنه المستخدم وأكّده الاختبار سابقًا: مقارنة الوقت التي كانت
//! تجعل كل وثيقة مؤهَّلة. الوقت هنا ثابت، فالنتيجة حتمية.

use govt_archive_system::db;
use govt_archive_system::db::retention::{disposal_candidate_ids, SECONDS_PER_YEAR};
use rusqlite::{params, Connection};

const NOW: i64 = 2_000_000_000; // وقت ثابت للاختبار

fn fresh() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    db::apply_migrations(&c).unwrap();
    c.execute("INSERT INTO users (id, username, password_hash, role, created_at) VALUES ('u','u','h','admin',1)", []).unwrap();
    c.execute("INSERT INTO document_types (id, name, created_at, retention_years) VALUES ('fin','مالية',1,10)", []).unwrap();
    c.execute("INSERT INTO document_types (id, name, created_at, retention_years) VALUES ('perm','دائمة',1,NULL)", []).unwrap();
    c
}

/// وثيقة بعمر محدَّد (بالسنوات) ونوع محدَّد ومجمّدة أو لا
fn add(c: &Connection, id: &str, type_id: Option<&str>, age_years: f64, hold: bool) {
    let created = NOW - (age_years * SECONDS_PER_YEAR as f64) as i64;
    c.execute(
        "INSERT INTO documents (id, title, file_hash, file_path, file_size, document_type_id,
                                created_at, legal_hold, origin_node_id)
         VALUES (?1, ?1, 'h', '/x', 1, ?2, ?3, ?4, 'n')",
        params![id, type_id, created, hold as i64],
    )
    .unwrap();
}

#[test]
fn expired_document_is_a_candidate() {
    let c = fresh();
    add(&c, "old", Some("fin"), 11.0, false);
    assert_eq!(disposal_candidate_ids(&c, NOW).unwrap(), vec!["old"]);
}

/// الخلل الأصلي: وثيقة حديثة لا يجب أن تظهر كمؤهَّلة (كانت تظهر كلها قبل الإصلاح)
#[test]
fn recent_document_is_not_a_candidate() {
    let c = fresh();
    add(&c, "new", Some("fin"), 2.0, false);
    assert!(disposal_candidate_ids(&c, NOW).unwrap().is_empty());
}

#[test]
fn legal_hold_blocks_disposal_even_when_expired() {
    let c = fresh();
    add(&c, "held", Some("fin"), 50.0, true);
    assert!(disposal_candidate_ids(&c, NOW).unwrap().is_empty());
}

#[test]
fn permanent_type_is_never_a_candidate() {
    let c = fresh();
    add(&c, "forever", Some("perm"), 500.0, false);
    assert!(disposal_candidate_ids(&c, NOW).unwrap().is_empty());
}

#[test]
fn document_without_type_is_never_a_candidate() {
    let c = fresh();
    add(&c, "untyped", None, 500.0, false);
    assert!(disposal_candidate_ids(&c, NOW).unwrap().is_empty());
}

#[test]
fn already_disposed_or_deleted_are_excluded() {
    let c = fresh();
    add(&c, "d1", Some("fin"), 20.0, false);
    add(&c, "d2", Some("fin"), 20.0, false);
    c.execute("UPDATE documents SET disposed_at = 1 WHERE id = 'd1'", []).unwrap();
    c.execute("UPDATE documents SET is_deleted = 1 WHERE id = 'd2'", []).unwrap();
    assert!(disposal_candidate_ids(&c, NOW).unwrap().is_empty());
}

/// الحدّ الفاصل بالضبط: بعد 10 سنوات تمامًا ليست مؤهَّلة، وبعدها بثانية نعم
#[test]
fn boundary_is_exact() {
    let c = fresh();
    let created_exact = NOW - 10 * SECONDS_PER_YEAR;
    c.execute(
        "INSERT INTO documents (id, title, file_hash, file_path, file_size, document_type_id,
                                created_at, origin_node_id)
         VALUES ('edge','edge','h','/x',1,'fin',?1,'n')",
        params![created_exact],
    ).unwrap();
    assert!(disposal_candidate_ids(&c, NOW).unwrap().is_empty(), "عند الحد تمامًا: ليست مؤهَّلة");
    assert_eq!(disposal_candidate_ids(&c, NOW + 1).unwrap(), vec!["edge"], "بعد ثانية: مؤهَّلة");
}
