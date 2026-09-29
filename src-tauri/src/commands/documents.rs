use crate::commands::auth::{require_role, SessionState, ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER};
use crate::crypto;
use crate::db::{DbState, VaultKeyState};
use crate::sync::{store as sync_store, SyncError, SyncState};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use tauri::{Manager, State};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub category_id: Option<String>,
    pub file_hash: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub has_ocr: bool,
}

#[derive(Debug, Deserialize)]
pub struct NewDocumentInput {
    pub title: String,
    pub category_id: Option<String>,
    /// المحتوى الخام للملف بصيغة base64 (قادم من الواجهة الأمامية)
    pub file_base64: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum ArchiveError {
    #[error("خطأ في قاعدة البيانات: {0}")]
    Db(String),
    #[error("خطأ في نظام الملفات: {0}")]
    Io(String),
    #[error("بيانات غير صالحة: {0}")]
    Invalid(String),
}

impl From<SyncError> for ArchiveError {
    fn from(e: SyncError) -> Self {
        ArchiveError::Db(e.to_string())
    }
}

impl From<rusqlite::Error> for ArchiveError {
    fn from(e: rusqlite::Error) -> Self {
        ArchiveError::Db(e.to_string())
    }
}

/// يُرجع اتصال قاعدة البيانات الحالي، أو خطأ واضح إن كانت الخزنة ما تزال مقفلة
macro_rules! require_conn {
    ($db:expr) => {{
        let guard = $db.0.lock().unwrap();
        match guard.as_ref() {
            Some(_) => guard,
            None => return Err(ArchiveError::Invalid("الخزنة مقفلة — افتحها أولًا".into())),
        }
    }};
}

/// إضافة وثيقة جديدة: تشفّر الملف بـ AES-256-GCM (مفتاح الخزنة) قبل تخزينه،
/// ثم تسجّل الميتاداتا. مسموح فقط لـ admin و archivist.
#[tauri::command]
pub fn add_document(
    app_handle: tauri::AppHandle,
    db: State<DbState>,
    vault_key: State<VaultKeyState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    input: NewDocumentInput,
) -> Result<Document, ArchiveError> {
    let user = require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST])?;

    use base64::{engine::general_purpose, Engine as _};
    let bytes = general_purpose::STANDARD
        .decode(&input.file_base64)
        .map_err(|e| ArchiveError::Invalid(format!("فشل فك ترميز base64: {e}")))?;

    // Hash محتوى الملف الأصلي (plaintext) — يُستخدم للتحقق من التكامل ولمنع التكرار،
    // وليس مرتبطًا بالتشفير نفسه (كل تشفير يستخدم Nonce عشوائيًا مختلفًا)
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());

    // OCR على المحتوى الصافي قبل التشفير — لا حاجة لفك تشفير لاحقًا لاستخراج النص.
    // يعمل بصمت (None) إن لم يكن Tesseract مثبَّتًا أو الملف ليس صورة.
    let ocr_text = crate::ocr::extract_text(&bytes, input.mime_type.as_deref());

    let key = vault_key
        .0
        .lock()
        .unwrap()
        .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| ArchiveError::Io(format!("تعذر تحديد مجلد بيانات التطبيق: {e}")))?;
    let vault = crate::db::vault_dir(&app_data_dir);

    let sub_dir = vault.join(&hash[0..2]);
    fs::create_dir_all(&sub_dir).map_err(|e| ArchiveError::Io(e.to_string()))?;
    let file_path = sub_dir.join(&hash);

    if !file_path.exists() {
        // تخزين: [Nonce (12 بايت)][النص المشفّر] في ملف واحد
        let (ciphertext, nonce) = crypto::encrypt(&key, &bytes);
        let mut out = Vec::with_capacity(nonce.len() + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        fs::write(&file_path, &out).map_err(|e| ArchiveError::Io(e.to_string()))?;
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();

    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();
    let node = sync_store::node_id(conn)?;

    conn.execute(
        "INSERT INTO documents
            (id, title, category_id, file_hash, file_path, file_size, mime_type, ocr_text, created_by, created_at, origin_node_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            input.title,
            input.category_id,
            hash,
            file_path.to_string_lossy().to_string(),
            bytes.len() as i64,
            input.mime_type,
            ocr_text,
            user.id,
            now,
            node,
        ],
    )?;

    conn.execute(
        "INSERT INTO audit_log (id, document_id, user_id, action, timestamp, node_id)
         VALUES (?1, ?2, ?3, 'created', ?4, 'local')",
        params![Uuid::new_v4().to_string(), id, user.id, now],
    )?;

    // سجل المزامنة: حالة الوثيقة كاملة (الملف نفسه يُنقل منفصلًا عبر hash)
    sync_store::log_local_event(
        conn,
        "document",
        &id,
        "upsert",
        &serde_json::json!({
            "id": id, "title": input.title, "category_id": input.category_id,
            "file_hash": hash, "file_size": bytes.len(), "mime_type": input.mime_type,
            "ocr_text": ocr_text, "created_by": user.id, "created_at": now, "updated_at": null,
            "origin_node_id": node, "is_deleted": 0
        }),
    )?;
    sync.poke();

    Ok(Document {
        id,
        title: input.title,
        category_id: input.category_id,
        file_hash: hash,
        file_size: bytes.len() as i64,
        mime_type: input.mime_type,
        created_at: now,
        updated_at: None,
        has_ocr: ocr_text.is_some(),
    })
}

/// هل Tesseract OCR متاح على هذا الجهاز؟ يُستخدم لعرض ملاحظة في الواجهة إن لم يكن مثبَّتًا.
#[tauri::command]
pub fn ocr_status() -> bool {
    crate::ocr::is_available()
}

/// جلب قائمة الوثائق (غير المحذوفة) — أي مستخدم مسجَّل دخوله يمكنه القراءة
#[tauri::command]
pub fn list_documents(
    db: State<DbState>,
    session: State<SessionState>,
) -> Result<Vec<Document>, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, title, category_id, file_hash, file_size, mime_type, created_at, updated_at,
                (ocr_text IS NOT NULL AND ocr_text <> '')
         FROM documents WHERE is_deleted = 0 ORDER BY created_at DESC",
    )?;
    let docs = stmt
        .query_map([], |row| {
            Ok(Document {
                id: row.get(0)?, title: row.get(1)?, category_id: row.get(2)?,
                file_hash: row.get(3)?, file_size: row.get(4)?, mime_type: row.get(5)?,
                created_at: row.get(6)?, updated_at: row.get(7)?, has_ocr: row.get(8)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(docs)
}

/// بحث نصي كامل (FTS5) — لأي مستخدم مسجَّل دخوله
#[tauri::command]
pub fn search_documents(
    db: State<DbState>,
    session: State<SessionState>,
    query: String,
) -> Result<Vec<Document>, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let mut stmt = conn.prepare(
        "SELECT d.id, d.title, d.category_id, d.file_hash, d.file_size, d.mime_type, d.created_at, d.updated_at,
                (d.ocr_text IS NOT NULL AND d.ocr_text <> '')
         FROM documents d JOIN documents_fts fts ON d.rowid = fts.rowid
         WHERE documents_fts MATCH ?1 AND d.is_deleted = 0 ORDER BY rank",
    )?;
    let docs = stmt
        .query_map(params![format!("{}*", query)], |row| {
            Ok(Document {
                id: row.get(0)?, title: row.get(1)?, category_id: row.get(2)?,
                file_hash: row.get(3)?, file_size: row.get(4)?, mime_type: row.get(5)?,
                created_at: row.get(6)?, updated_at: row.get(7)?, has_ocr: row.get(8)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(docs)
}

/// فك تشفير محتوى وثيقة لعرضه/تنزيله — متاح لكل الأدوار (قراءة)، ويُسجَّل في audit_log
#[tauri::command]
pub fn get_document_file(
    app_handle: tauri::AppHandle,
    db: State<DbState>,
    vault_key: State<VaultKeyState>,
    session: State<SessionState>,
    id: String,
) -> Result<String, ArchiveError> {
    let user = require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER])?;
    let _ = &app_handle; // محجوزة لاستخدام مستقبلي (مثل مسارات نسبية)

    let key = vault_key
        .0
        .lock()
        .unwrap()
        .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;

    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let file_path: String = conn
        .query_row(
            "SELECT file_path FROM documents WHERE id = ?1 AND is_deleted = 0",
            params![id],
            |row| row.get(0),
        )
        .map_err(|_| ArchiveError::Invalid("الوثيقة غير موجودة".into()))?;

    let raw = fs::read(&file_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ArchiveError::Invalid("لم يصل ملف هذه الوثيقة إلى هذا الجهاز بعد — بانتظار المزامنة".into())
        } else {
            ArchiveError::Io(e.to_string())
        }
    })?;
    if raw.len() < crypto::NONCE_LEN {
        return Err(ArchiveError::Invalid("ملف تالف".into()));
    }
    let (nonce, ciphertext) = raw.split_at(crypto::NONCE_LEN);
    let plaintext = crypto::decrypt(&key, ciphertext, nonce).map_err(ArchiveError::Invalid)?;

    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO audit_log (id, document_id, user_id, action, timestamp, node_id)
         VALUES (?1, ?2, ?3, 'viewed', ?4, 'local')",
        params![Uuid::new_v4().to_string(), id, user.id, now],
    )?;

    use base64::{engine::general_purpose, Engine as _};
    Ok(general_purpose::STANDARD.encode(plaintext))
}

/// حذف منطقي (Soft Delete) — لا حذف نهائي أبدًا. مسموح فقط لـ admin و archivist.
#[tauri::command]
pub fn delete_document(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    id: String,
) -> Result<(), ArchiveError> {
    let user = require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST])?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();
    let now = Utc::now().timestamp();

    conn.execute(
        "UPDATE documents SET is_deleted = 1, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    conn.execute(
        "INSERT INTO audit_log (id, document_id, user_id, action, timestamp, node_id)
         VALUES (?1, ?2, ?3, 'deleted', ?4, 'local')",
        params![Uuid::new_v4().to_string(), id, user.id, now],
    )?;
    sync_store::log_local_event(
        conn,
        "document",
        &id,
        "delete",
        &serde_json::json!({ "id": id, "updated_at": now }),
    )?;
    sync.poke();
    Ok(())
}
