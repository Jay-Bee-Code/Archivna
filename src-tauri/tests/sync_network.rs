//! اختبار تكامل للمزامنة عبر الشبكة الفعلية بين عقدتين.
//!
//! عقدتان بقاعدتين مستقلتين، وخادم WebSocket حقيقي على localhost، وتبادل كامل:
//! أحداث الوثائق والفئات، وملف مشفّر بحجم يتطلب عدة أجزاء. يُتحقَّق من النتيجة
//! في الجهازين، ومن أن الملف يُفكّ بمفتاح المستلم وحده.

use govt_archive_system::crypto;
use govt_archive_system::db;
use govt_archive_system::sync::net::{run_client, serve, SyncCore};
use govt_archive_system::sync::store;
use rusqlite::{params, Connection};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::watch;

fn node(name: &str, sync_key: [u8; 32]) -> SyncCore {
    let conn = Connection::open_in_memory().unwrap();
    db::apply_migrations(&conn).unwrap();
    let (id, _) = store::ensure_node_config(&conn).unwrap();
    conn.execute("UPDATE node_config SET node_name = ?1", params![name]).unwrap();
    let dir: PathBuf = std::env::temp_dir().join(format!("netsync-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    SyncCore {
        db: Arc::new(Mutex::new(Some(conn))),
        vault_key: Arc::new(Mutex::new(Some(crypto::generate_vault_key()))),
        vault_dir: dir,
        sync_key,
        node_id: id,
        node_name: name.into(),
        session_lock: Arc::new(tokio::sync::Mutex::new(())),
    }
}

async fn serve_node(core: &SyncCore) -> (SocketAddr, watch::Sender<bool>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = watch::channel(false);
    tokio::spawn(serve(core.clone(), listener, rx, Arc::new(|_| {})));
    (addr, tx)
}

/// يضيف فئة محلية ويسجّل حدثها (كما يفعل الأمر الحقيقي)
fn add_category(core: &SyncCore, id: &str, name: &str) {
    let g = core.db.lock().unwrap();
    let c = g.as_ref().unwrap();
    c.execute("INSERT INTO categories (id, name, created_at) VALUES (?1, ?2, 1)", params![id, name]).unwrap();
    store::log_local_event(
        c, "category", id, "upsert",
        &json!({ "id": id, "name": name, "parent_id": null, "created_at": 1 }),
    )
    .unwrap();
}

/// يضيف وثيقة بملف مشفّر فعلي، ويسجّل حدثها بالحالة الكاملة
fn add_document(core: &SyncCore, id: &str, bytes: &[u8]) -> String {
    let hash = format!("{:x}", Sha256::digest(bytes));
    let key = core.vault_key.lock().unwrap().unwrap();
    store::write_blob_from_plain(&core.vault_dir, &key, &hash, bytes).unwrap();
    let path = store::blob_path(&core.vault_dir, &hash).unwrap();
    let g = core.db.lock().unwrap();
    let c = g.as_ref().unwrap();
    c.execute(
        "INSERT INTO documents (id, title, file_hash, file_path, file_size, created_at, origin_node_id)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        params![id, id, hash, path.to_string_lossy().to_string(), bytes.len() as i64, core.node_id],
    )
    .unwrap();
    store::log_local_event(
        c, "document", id, "upsert",
        &json!({
            "id": id, "title": id, "category_id": null, "file_hash": hash, "file_size": bytes.len(),
            "mime_type": null, "ocr_text": null, "department_id": null, "document_type_id": null,
            "registry_number": null, "confidentiality_level": 1, "status": "approved", "metadata": null,
            "legal_hold": 0, "disposed_at": null, "physical_location": null, "physical_status": "none",
            "borrowed_by": null, "borrowed_at": null, "correspondent_id": null, "created_by": null,
            "created_at": 1, "updated_at": null, "origin_node_id": core.node_id, "is_deleted": 0
        }),
    )
    .unwrap();
    hash
}

fn count(core: &SyncCore, sql: &str) -> i64 {
    let g = core.db.lock().unwrap();
    g.as_ref().unwrap().query_row(sql, [], |r| r.get(0)).unwrap()
}

fn read_plain(core: &SyncCore, hash: &str) -> Vec<u8> {
    let key = core.vault_key.lock().unwrap().unwrap();
    store::read_blob_plain(&core.vault_dir, &key, hash).unwrap()
}

/// بيانات عشوائية بحجم يتجاوز جزءًا واحدًا (CHUNK = 192KB) لإجبار تقسيم الملف
fn large_bytes(n: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut v = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut v);
    v
}

#[tokio::test(flavor = "multi_thread")]
async fn two_nodes_exchange_events_and_an_encrypted_multi_chunk_file() {
    let key = crypto::derive_sync_key("org-secret-123");
    let a = node("A", key);
    let b = node("B", key);

    add_category(&a, "cat-1", "قرارات");
    let big = large_bytes(700 * 1024 + 13); // أكثر من جزء واحد
    let hash = add_document(&a, "doc-1", &big);

    let (addr_b, _s) = serve_node(&b).await;
    let rep = run_client(&a, addr_b).await.expect("المزامنة يجب أن تنجح");

    assert_eq!(rep.events_sent, 2, "فئة + وثيقة");
    assert_eq!(rep.files_sent, 1, "ملف واحد أُرسل");
    assert_eq!(rep.peer_name, "B");

    // الحالة وصلت كما هي
    assert_eq!(count(&b, "SELECT COUNT(*) FROM categories WHERE name = 'قرارات'"), 1);
    assert_eq!(count(&b, "SELECT COUNT(*) FROM documents WHERE is_deleted = 0"), 1);
    // الملف وصل كاملًا ومطابقًا بعد فك التشفير بمفتاح المستلم
    assert_eq!(read_plain(&b, &hash), big, "محتوى الملف يجب أن يطابق الأصل بايتًا بايتًا");

    // الملف مشفّر على B بمفتاحه هو: مفتاح A لا يفكّه
    let a_key = a.vault_key.lock().unwrap().unwrap();
    assert!(
        store::read_blob_plain(&b.vault_dir, &a_key, &hash).is_err(),
        "الملف على B يجب أن يكون مشفّرًا بمفتاح B وحده"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn second_sync_without_changes_transfers_nothing() {
    let key = crypto::derive_sync_key("org-secret-123");
    let a = node("A", key);
    let b = node("B", key);
    add_category(&a, "cat-1", "x");
    add_document(&a, "doc-1", b"small");

    let (addr_b, _s) = serve_node(&b).await;
    run_client(&a, addr_b).await.unwrap();
    let rep2 = run_client(&a, addr_b).await.unwrap();
    assert_eq!(
        (rep2.events_sent, rep2.events_received, rep2.files_sent, rep2.files_received),
        (0, 0, 0, 0),
        "مزامنة ثانية بلا تغييرات لا تنقل شيئًا"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn wrong_sync_key_is_rejected_and_leaks_nothing() {
    let a = node("A", crypto::derive_sync_key("right-key-123"));
    let intruder = node("E", crypto::derive_sync_key("WRONG-key-123"));
    add_category(&a, "cat-1", "سري");

    let (addr_a, _s) = serve_node(&a).await;
    let res = run_client(&intruder, addr_a).await;
    assert!(res.is_err(), "مفتاح خاطئ يجب أن يُرفض");
    assert_eq!(count(&intruder, "SELECT COUNT(*) FROM categories"), 0, "لا تسرّب لأي بيانات");
}
