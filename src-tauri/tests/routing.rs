//! اختبار تكامل لصندوق الإحالات الحقيقي (db::routing::inbox_for).
//!
//! يتحقق من: التصفية حسب المستلم، استبعاد الوثائق المحذوفة، الترتيب الأحدث أولًا،
//! وربط اسم المُحيل من جدول المستخدمين.

use govt_archive_system::db;
use govt_archive_system::db::routing::inbox_for;
use rusqlite::{params, Connection};

fn fresh() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    db::apply_migrations(&c).unwrap();
    for (id, name) in [("alice", "alice"), ("bob", "bob"), ("carol", "carol")] {
        c.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?1, ?2, 'h', 'archivist', 1)",
            params![id, name],
        )
        .unwrap();
    }
    for (id, title) in [("doc-1", "وثيقة أولى"), ("doc-2", "وثيقة ثانية"), ("doc-3", "وثيقة محذوفة")] {
        c.execute(
            "INSERT INTO documents (id, title, file_hash, file_path, file_size, created_at, origin_node_id)
             VALUES (?1, ?2, 'h', '/x', 1, 1, 'n')",
            params![id, title],
        )
        .unwrap();
    }
    c
}

fn route(c: &Connection, id: &str, doc: &str, to: &str, by: &str, at: i64, note: Option<&str>) {
    c.execute(
        "INSERT INTO document_routing (id, document_id, routed_to_user_id, routed_by, routed_at, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, doc, to, by, at, note],
    )
    .unwrap();
}

#[test]
fn inbox_returns_only_recipient_items_newest_first_without_deleted() {
    let c = fresh();
    route(&c, "r1", "doc-1", "alice", "bob", 100, None);
    route(&c, "r2", "doc-2", "alice", "carol", 200, Some("عاجل"));   // الأحدث
    route(&c, "r3", "doc-1", "bob", "alice", 300, None);             // لشخص آخر
    route(&c, "r4", "doc-3", "alice", "bob", 400, None);             // وثيقة محذوفة
    c.execute("UPDATE documents SET is_deleted = 1 WHERE id = 'doc-3'", []).unwrap();

    let rows = inbox_for(&c, "alice").unwrap();
    assert_eq!(rows.len(), 2, "alice لها إحالتان صالحتان فقط");
    assert_eq!(rows[0].id, "r2", "الأحدث أولًا");
    assert_eq!(rows[1].id, "r1");
    assert_eq!(rows[0].routed_by_username.as_deref(), Some("carol"), "اسم المُحيل من جدول المستخدمين");
    assert_eq!(rows[0].note.as_deref(), Some("عاجل"));
    assert_eq!(rows[0].document_title, "وثيقة ثانية");
}

#[test]
fn empty_inbox_for_user_with_no_routings() {
    let c = fresh();
    route(&c, "r1", "doc-1", "bob", "alice", 100, None);
    assert!(inbox_for(&c, "carol").unwrap().is_empty());
}

/// المُحيل مُعطَّل منطقيًا (is_active=0) لا محذوف: صفه يبقى بسبب المفتاح الأجنبي،
/// فتبقى الإحالة في صندوق المستلم ويظهر اسم المُحيل. هذا يحمي السجل الإداري الدائم.
/// (ملاحظة: إحالة من مستخدم غير موجودة أصلًا يرفضها المخطط بقيد FOREIGN KEY —
/// وهذا سلوك مقصود، لا يُختبر هنا كحالة صالحة.)
#[test]
fn routing_from_disabled_user_still_shows_with_their_name() {
    let c = fresh();
    c.execute("UPDATE users SET is_active = 0 WHERE id = 'bob'", []).unwrap();
    route(&c, "r1", "doc-1", "alice", "bob", 100, None);
    let rows = inbox_for(&c, "alice").unwrap();
    assert_eq!(rows.len(), 1, "الإحالة تبقى حتى لو عُطِّل المُحيل");
    assert_eq!(rows[0].routed_by_username.as_deref(), Some("bob"));
}

/// الإحالة من مستخدم غير موجودة مرفوضة من المخطط نفسه (سلامة مرجعية)
#[test]
fn routing_from_nonexistent_user_is_rejected_by_schema() {
    let c = fresh();
    let r = c.execute(
        "INSERT INTO document_routing (id, document_id, routed_to_user_id, routed_by, routed_at)
         VALUES ('rx', 'doc-1', 'alice', 'ghost', 100)",
        [],
    );
    assert!(r.is_err(), "المخطط يجب أن يرفض مُحيلًا غير موجود");
}
