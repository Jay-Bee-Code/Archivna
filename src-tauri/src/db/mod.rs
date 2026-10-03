use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// اتصال قاعدة البيانات — None قبل فتح الخزنة بالعبارة السرية
pub struct DbState(pub Arc<Mutex<Option<Connection>>>);

/// مفتاح تشفير الملفات (Vault Master Key)، يُحفظ في الذاكرة فقط بعد فتح الخزنة
/// — لا يُكتب على القرص أبدًا بصيغته الصافية
pub struct VaultKeyState(pub Arc<Mutex<Option<[u8; 32]>>>);

pub fn db_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("archive.db")
}

/// هل توجد خزنة (قاعدة بيانات) على هذا الجهاز أصلًا؟
pub fn vault_exists(app_data_dir: &PathBuf) -> bool {
    db_path(app_data_dir).exists()
}

// ============================================================
// نظام إصدارات المخطط (Schema Migrations)
//
// كل تغيير مستقبلي على المخطط يضاف كملف SQL مرقَّم جديد هنا، لا كتعديل
// على ملفات موجودة — الأرشيف قد يحمل بيانات حقيقية، وتعديل migration
// طُبِّق فعلًا على أجهزة أخرى يكسر التقارب بين الأجهزة. `schema_version`
// يضمن أن كل migration يُطبَّق مرة واحدة فقط بترتيب صارم على كل جهاز.
// ============================================================

struct Migration {
    version: i32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration { version: 1, sql: include_str!("migrations/0001_initial.sql") },
    Migration { version: 2, sql: include_str!("migrations/0002_org_structure.sql") },
    Migration { version: 3, sql: include_str!("migrations/0003_retention_physical.sql") },
];

pub fn apply_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);")?;
    let current: i32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |r| r.get(0),
    )?;
    for m in MIGRATIONS {
        if m.version > current {
            conn.execute_batch(m.sql)?;
            conn.execute("INSERT INTO schema_version (version) VALUES (?1)", params![m.version])?;
        }
    }
    Ok(())
}

/// يفتح قاعدة بيانات مشفّرة بـ SQLCipher بالعبارة السرية المعطاة.
/// ينشئ القاعدة والمخطط تلقائيًا إن لم تكن موجودة (أول تشغيل)، ويرفع أي قاعدة
/// موجودة تدريجيًا لآخر إصدار مخطط معروف عبر apply_migrations.
pub fn open_encrypted_db(app_data_dir: &PathBuf, passphrase: &str) -> rusqlite::Result<Connection> {
    std::fs::create_dir_all(app_data_dir).expect("فشل إنشاء مجلد بيانات التطبيق");
    let conn = Connection::open(db_path(app_data_dir))?;

    // مفتاح SQLCipher — يجب تعيينه فورًا كأول عملية بعد فتح الاتصال
    conn.pragma_update(None, "key", passphrase)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // أول استعلام حقيقي: ينجح فقط إذا كانت العبارة السرية صحيحة
    apply_migrations(&conn)?;

    Ok(conn)
}

/// مجلد الخزنة حيث تُخزَّن الملفات الفعلية (مشفّرة بـ AES-256-GCM)
pub fn vault_dir(app_data_dir: &PathBuf) -> PathBuf {
    let dir = app_data_dir.join("vault");
    std::fs::create_dir_all(&dir).expect("فشل إنشاء مجلد الخزنة");
    dir
}
