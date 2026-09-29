use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// اتصال قاعدة البيانات — None قبل فتح الخزنة بالعبارة السرية
pub struct DbState(pub Arc<Mutex<Option<Connection>>>);

/// مفتاح تشفير الملفات (Vault Master Key)، يُحفظ في الذاكرة فقط بعد فتح الخزنة
/// — لا يُكتب على القرص أبدًا بصيغته الصافية
pub struct VaultKeyState(pub Arc<Mutex<Option<[u8; 32]>>>);

const SCHEMA: &str = include_str!("schema.sql");

pub fn db_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("archive.db")
}

/// هل توجد خزنة (قاعدة بيانات) على هذا الجهاز أصلًا؟ يحدد الواجهة الأمامية
/// إن كانت تعرض شاشة "إنشاء عبارة سرية" أو "أدخل العبارة السرية"
pub fn vault_exists(app_data_dir: &PathBuf) -> bool {
    db_path(app_data_dir).exists()
}

/// يفتح قاعدة بيانات مشفّرة بـ SQLCipher بالعبارة السرية المعطاة.
/// ينشئ القاعدة والمخطط تلقائيًا إن لم تكن موجودة (أول تشغيل على هذا الجهاز).
/// عبارة سرية خاطئة على قاعدة موجودة تُسبب فشل execute_batch (القاعدة تبدو تالفة لأنها ما تزال مشفّرة).
pub fn open_encrypted_db(app_data_dir: &PathBuf, passphrase: &str) -> rusqlite::Result<Connection> {
    std::fs::create_dir_all(app_data_dir).expect("فشل إنشاء مجلد بيانات التطبيق");
    let conn = Connection::open(db_path(app_data_dir))?;

    // مفتاح SQLCipher — يجب تعيينه فورًا كأول عملية بعد فتح الاتصال
    conn.pragma_update(None, "key", passphrase)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // أول استعلام حقيقي: ينجح فقط إذا كانت العبارة السرية صحيحة
    conn.execute_batch(SCHEMA)?;

    Ok(conn)
}

/// مجلد الخزنة حيث تُخزَّن الملفات الفعلية (مشفّرة بـ AES-256-GCM لاحقًا)
pub fn vault_dir(app_data_dir: &PathBuf) -> PathBuf {
    let dir = app_data_dir.join("vault");
    std::fs::create_dir_all(&dir).expect("فشل إنشاء مجلد الخزنة");
    dir
}
