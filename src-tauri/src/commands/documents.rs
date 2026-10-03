use crate::commands::auth::{
    bypasses_visibility_filter, require_role, SessionState, ROLE_ADMIN, ROLE_ARCHIVIST,
    ROLE_REVIEWER, ROLE_VIEWER,
};
use crate::crypto;
use crate::db::{DbState, VaultKeyState};
use crate::sync::{store as sync_store, SyncError, SyncState};
use chrono::{Datelike, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use tauri::{Manager, State};
use uuid::Uuid;

const ALL_ROLES: &[&str] = &[ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER];
/// الحالات المسموحة لدورة حياة الوثيقة — راجع القسم 5 من اقتراح التنظيم الإداري
const VALID_STATUSES: &[&str] = &["draft", "in_review", "approved", "archived", "superseded", "disposed"];

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
    pub department_id: Option<String>,
    pub document_type_id: Option<String>,
    pub registry_number: Option<String>,
    pub confidentiality_level: i64,
    pub status: String,
    pub legal_hold: bool,
    pub disposed_at: Option<i64>,
    pub physical_location: Option<String>,
    pub physical_status: String,
    pub borrowed_by: Option<String>,
    pub borrowed_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct NewDocumentInput {
    pub title: String,
    pub category_id: Option<String>,
    /// المحتوى الخام للملف بصيغة base64 (قادم من الواجهة الأمامية)
    pub file_base64: String,
    pub mime_type: Option<String>,
    pub department_id: Option<String>,
    pub document_type_id: Option<String>,
    pub confidentiality_level: Option<i64>,
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

impl From<rusqlite::Error> for ArchiveError {
    fn from(e: rusqlite::Error) -> Self {
        ArchiveError::Db(e.to_string())
    }
}

impl From<SyncError> for ArchiveError {
    fn from(e: SyncError) -> Self {
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

pub(crate) const DOC_COLUMNS: &str = "id, title, category_id, file_hash, file_size, mime_type,
    created_at, updated_at, (ocr_text IS NOT NULL AND ocr_text <> ''),
    department_id, document_type_id, registry_number, confidentiality_level, status,
    legal_hold, disposed_at, physical_location, physical_status, borrowed_by, borrowed_at";

/// يبني حالة الوثيقة كاملة كـ JSON لإرسالها عبر المزامنة. **إلزامي** لأي حدث
/// "document"/"upsert" — محرك المزامنة (sync/store.rs) يتطلب الحالة كاملة في
/// كل حدث upsert (وليس تغييرًا جزئيًا)، لأن الحدث الفائز بترتيب Last-Writer-Wins
/// قد لا يكون آخر حدث وصل فعليًا، فيُعاد بناء الصف كاملًا من ذلك الحدث وحده.
/// إرسال حقول جزئية فقط (كما فعلت نسخة مبكرة من update_document_status/
/// set_legal_hold/update_physical/dispose_document) يُسقط الحقول الغائبة
/// إلى NULL لدى الطرف المستلم، ويُفشل المزامنة بالكامل إن غاب title/file_hash
/// (حقلان إلزاميان) — اختبار تكامل حقيقي كشف هذا الخلل وأثبت الإصلاح.
pub(crate) fn full_document_payload(conn: &Connection, id: &str) -> Result<serde_json::Value, ArchiveError> {
    conn.query_row(
        "SELECT title, category_id, file_hash, file_size, mime_type, ocr_text,
                department_id, document_type_id, registry_number, confidentiality_level, status, metadata,
                legal_hold, disposed_at, physical_location, physical_status, borrowed_by, borrowed_at,
                created_by, created_at, updated_at, origin_node_id, is_deleted
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
            }))
        },
    )
    .map_err(|_| ArchiveError::Invalid("الوثيقة غير موجودة".into()))
}

pub(crate) fn row_to_document(row: &rusqlite::Row) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?, title: row.get(1)?, category_id: row.get(2)?,
        file_hash: row.get(3)?, file_size: row.get(4)?, mime_type: row.get(5)?,
        created_at: row.get(6)?, updated_at: row.get(7)?, has_ocr: row.get(8)?,
        department_id: row.get(9)?, document_type_id: row.get(10)?,
        registry_number: row.get(11)?, confidentiality_level: row.get(12)?, status: row.get(13)?,
        legal_hold: row.get(14)?, disposed_at: row.get(15)?, physical_location: row.get(16)?,
        physical_status: row.get(17)?, borrowed_by: row.get(18)?, borrowed_at: row.get(19)?,
    })
}

/// شرط SQL يقيّد الرؤية حسب القسم ومستوى الاطّلاع لغير المدير — يُلحَق بأي استعلام
/// وثائق. المعاملان الإضافيان بالترتيب: مستوى الاطّلاع، ثم القسم (مرتين).
pub(crate) fn visibility_sql(role: &str) -> &'static str {
    if bypasses_visibility_filter(role) {
        ""
    } else {
        " AND confidentiality_level <= :A AND (:B IS NULL OR department_id IS NULL OR department_id = :B)"
    }
}

/// رقم القيد: {السنة}/{رمز القسم}/{تسلسل محلي للجهاز والقسم والسنة}-{مُعرّف الجهاز القصير}.
///
/// بلا خادم مركزي، جهازان بلا اتصال سيولّدان حتمًا نفس "التسلسل النظيف" في نفس اليوم؛
/// لذلك المعرّف القصير إلزامي لضمان عدم التصادم أبدًا — على حساب التسلسل المطلق بلا فجوات.
fn generate_registry_number(
    conn: &Connection,
    node_id: &str,
    department_id: &str,
) -> Result<String, ArchiveError> {
    let year = Utc::now().year();
    let dept_code: Option<String> = conn
        .query_row("SELECT code FROM departments WHERE id = ?1", params![department_id], |r| r.get(0))
        .ok();
    let code = dept_code.filter(|c| !c.trim().is_empty()).unwrap_or_else(|| "GEN".to_string());

    let seq: i64 = conn.query_row(
        "INSERT INTO registry_counters (node_id, department_id, year, next_seq) VALUES (?1, ?2, ?3, 2)
         ON CONFLICT(node_id, department_id, year) DO UPDATE SET next_seq = next_seq + 1
         RETURNING next_seq - 1",
        params![node_id, department_id, year],
        |r| r.get(0),
    )?;

    let node_suffix = node_suffix_for(node_id);
    Ok(format!("{year}/{code}/{seq:04}-{node_suffix}"))
}

/// معرّف قصير مشتق بالهاش (وليس substring خام من UUID الجهاز) ليُستخدم داخل
/// رقم القيد. substring خام من UUID محفوف بخطر تصادم بنيوي (مثلًا لو تشارك
/// جهازان نفس البادئة النصية)؛ هاش SHA-256 يوزّع المعرّفات بانتظام حقيقي،
/// و6 خانات سداسية عشرية (~16.7 مليون احتمال) تجعل التصادم بين عشرات
/// الأجهزة عبر سنوات التشغيل مهملًا عمليًا.
fn node_suffix_for(node_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(node_id.as_bytes());
    let digest = hasher.finalize();
    format!("{:02X}{:02X}{:02X}", digest[0], digest[1], digest[2])
}

/// إضافة وثيقة جديدة: تشفّر الملف بـ AES-256-GCM (مفتاح الخزنة) قبل تخزينه،
/// تُولّد رقم قيد إن حُدِّد قسم، وتسجّل الميتاداتا. مسموح فقط لـ admin و archivist.
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
    let confidentiality = input.confidentiality_level.unwrap_or(1).clamp(1, 4);
    // القسم المحدَّد صراحةً، وإلا قسم المُنشئ (إن وُجد) كافتراض معقول
    let department_id = input.department_id.clone().or_else(|| user.department_id.clone());

    use base64::{engine::general_purpose, Engine as _};
    let bytes = general_purpose::STANDARD
        .decode(&input.file_base64)
        .map_err(|e| ArchiveError::Invalid(format!("فشل فك ترميز base64: {e}")))?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());

    // OCR على المحتوى الصافي قبل التشفير — لا حاجة لفك تشفير لاحقًا لاستخراج النص
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

    let registry_number = match &department_id {
        Some(dept) => Some(generate_registry_number(conn, &node, dept)?),
        None => None,
    };
    let status = "approved"; // راجع تعليق VALID_STATUSES: سير العمل الكامل (مسودة→مراجعة) مؤجَّل لمرحلة لاحقة

    conn.execute(
        "INSERT INTO documents
            (id, title, category_id, file_hash, file_path, file_size, mime_type, ocr_text,
             department_id, document_type_id, registry_number, confidentiality_level, status,
             created_by, created_at, origin_node_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            id, input.title, input.category_id, hash,
            file_path.to_string_lossy().to_string(), bytes.len() as i64, input.mime_type, ocr_text,
            department_id, input.document_type_id, registry_number, confidentiality, status,
            user.id, now, node,
        ],
    )?;

    conn.execute(
        "INSERT INTO audit_log (id, document_id, user_id, action, timestamp, node_id)
         VALUES (?1, ?2, ?3, 'created', ?4, 'local')",
        params![Uuid::new_v4().to_string(), id, user.id, now],
    )?;

    sync_store::log_local_event(
        conn, "document", &id, "upsert",
        &serde_json::json!({
            "id": id, "title": input.title, "category_id": input.category_id,
            "file_hash": hash, "file_size": bytes.len(), "mime_type": input.mime_type,
            "ocr_text": ocr_text, "created_by": user.id, "created_at": now, "updated_at": null,
            "origin_node_id": node, "is_deleted": 0,
            "department_id": department_id, "document_type_id": input.document_type_id,
            "registry_number": registry_number, "confidentiality_level": confidentiality,
            "status": status, "metadata": null
        }),
    )?;
    sync.poke();

    Ok(Document {
        id, title: input.title, category_id: input.category_id, file_hash: hash,
        file_size: bytes.len() as i64, mime_type: input.mime_type, created_at: now, updated_at: None,
        has_ocr: ocr_text.is_some(), department_id, document_type_id: input.document_type_id,
        registry_number, confidentiality_level: confidentiality, status: status.to_string(),
        legal_hold: false, disposed_at: None, physical_location: None,
        physical_status: "none".to_string(), borrowed_by: None, borrowed_at: None,
    })
}

/// هل Tesseract OCR متاح على هذا الجهاز؟
#[tauri::command]
pub fn ocr_status() -> bool {
    crate::ocr::is_available()
}

/// جلب قائمة الوثائق (غير المحذوفة)، مقيَّدة بالقسم ومستوى الاطّلاع لغير المدير
#[tauri::command]
pub fn list_documents(
    db: State<DbState>,
    session: State<SessionState>,
) -> Result<Vec<Document>, ArchiveError> {
    let user = require_role(&session, ALL_ROLES)?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let sql = format!(
        "SELECT {DOC_COLUMNS} FROM documents WHERE is_deleted = 0{} ORDER BY created_at DESC",
        visibility_sql(&user.role)
    );
    let mut stmt = conn.prepare(&sql)?;
    let docs = if bypasses_visibility_filter(&user.role) {
        stmt.query_map([], row_to_document)?.filter_map(Result::ok).collect()
    } else {
        stmt.query_map(
            rusqlite::named_params! { ":A": user.clearance_level, ":B": user.department_id },
            row_to_document,
        )?
        .filter_map(Result::ok)
        .collect()
    };
    Ok(docs)
}

/// بحث نصي كامل (FTS5)، مقيَّد بنفس قواعد الرؤية
#[tauri::command]
pub fn search_documents(
    db: State<DbState>,
    session: State<SessionState>,
    query: String,
) -> Result<Vec<Document>, ArchiveError> {
    let user = require_role(&session, ALL_ROLES)?;
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    // الأعمدة مُسبَّقة بـ d. يدويًا (بدل إعادة استخدام DOC_COLUMNS) لتفادي غموضها مع JOIN
    let sql = format!(
        "SELECT d.id, d.title, d.category_id, d.file_hash, d.file_size, d.mime_type,
                d.created_at, d.updated_at, (d.ocr_text IS NOT NULL AND d.ocr_text <> ''),
                d.department_id, d.document_type_id, d.registry_number, d.confidentiality_level, d.status,
                d.legal_hold, d.disposed_at, d.physical_location, d.physical_status, d.borrowed_by, d.borrowed_at
         FROM documents d JOIN documents_fts fts ON d.rowid = fts.rowid
         WHERE documents_fts MATCH :q AND d.is_deleted = 0{}
         ORDER BY rank",
        visibility_sql(&user.role)
    );
    let mut stmt = conn.prepare(&sql)?;
    let like = format!("{}*", query);
    let docs = if bypasses_visibility_filter(&user.role) {
        stmt.query_map(rusqlite::named_params! { ":q": like }, row_to_document)?
            .filter_map(Result::ok)
            .collect()
    } else {
        stmt.query_map(
            rusqlite::named_params! { ":q": like, ":A": user.clearance_level, ":B": user.department_id },
            row_to_document,
        )?
        .filter_map(Result::ok)
        .collect()
    };
    Ok(docs)
}

/// فك تشفير محتوى وثيقة لعرضه/تنزيله — يتحقق من قواعد الرؤية أولًا، ويُسجَّل في audit_log
#[tauri::command]
pub fn get_document_file(
    app_handle: tauri::AppHandle,
    db: State<DbState>,
    vault_key: State<VaultKeyState>,
    session: State<SessionState>,
    id: String,
) -> Result<String, ArchiveError> {
    let user = require_role(&session, ALL_ROLES)?;
    let _ = &app_handle;

    let key = vault_key
        .0
        .lock()
        .unwrap()
        .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;

    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();

    let (file_path, conf, dept): (String, i64, Option<String>) = conn
        .query_row(
            "SELECT file_path, confidentiality_level, department_id FROM documents WHERE id = ?1 AND is_deleted = 0",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| ArchiveError::Invalid("الوثيقة غير موجودة".into()))?;

    if !bypasses_visibility_filter(&user.role) {
        let dept_ok = user.department_id.is_none() || dept.is_none() || dept == user.department_id;
        if conf > user.clearance_level || !dept_ok {
            return Err(ArchiveError::Invalid("لا تملك صلاحية اطّلاع كافية لهذه الوثيقة".into()));
        }
    }

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

/// تغيير حالة الوثيقة ضمن دورة حياتها — للمراجع والمدير (وظيفة المراجع الفعلية)
#[tauri::command]
pub fn update_document_status(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    id: String,
    status: String,
) -> Result<(), ArchiveError> {
    let user = require_role(&session, &[ROLE_ADMIN, ROLE_REVIEWER])?;
    if !VALID_STATUSES.contains(&status.as_str()) {
        return Err(ArchiveError::Invalid(format!("حالة غير معروفة: {status}")));
    }
    let guard = require_conn!(db);
    let conn = guard.as_ref().unwrap();
    let now = Utc::now().timestamp();

    let affected = conn.execute(
        "UPDATE documents SET status = ?1, updated_at = ?2 WHERE id = ?3 AND is_deleted = 0",
        params![status, now, id],
    )?;
    if affected == 0 {
        return Err(ArchiveError::Invalid("الوثيقة غير موجودة".into()));
    }
    conn.execute(
        "INSERT INTO audit_log (id, document_id, user_id, action, timestamp, node_id, metadata)
         VALUES (?1, ?2, ?3, 'status_changed', ?4, 'local', ?5)",
        params![Uuid::new_v4().to_string(), id, user.id, now, status],
    )?;
    sync_store::log_local_event(conn, "document", &id, "upsert", &full_document_payload(conn, &id)?)?;
    sync.poke();
    Ok(())
}

/// حذف منطقي (Soft Delete) — لا حذف نهائي أبدًا. مسموح فقط لـ admin و archivist،
/// وضمن قواعد الرؤية نفسها (لا يحذف أرشيفيٌّ وثيقة قسم آخر لا يراه أصلًا).
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

    if !bypasses_visibility_filter(&user.role) {
        let (conf, dept): (i64, Option<String>) = conn.query_row(
            "SELECT confidentiality_level, department_id FROM documents WHERE id = ?1",
            params![id], |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let dept_ok = user.department_id.is_none() || dept.is_none() || dept == user.department_id;
        if conf > user.clearance_level || !dept_ok {
            return Err(ArchiveError::Invalid("لا تملك صلاحية لحذف هذه الوثيقة".into()));
        }
    }

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
    sync_store::log_local_event(conn, "document", &id, "delete", &serde_json::json!({ "id": id, "updated_at": now }))?;
    sync.poke();
    Ok(())
}
