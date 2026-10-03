//! طبقة بيانات المزامنة: سجل الأحداث + تطبيقها + الملفات (Blobs).
//! لا تعتمد على Tauri إطلاقًا، لذلك تُختبر منفردة.

use crate::crypto;
use crate::sync::SyncError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SyncEvent {
    pub origin_node_id: String,
    pub origin_seq: i64,
    pub entity_type: String, // document | category | user
    pub entity_id: String,
    pub operation: String, // upsert | delete
    pub payload: Value,
    pub lamport: i64,
    pub created_at: i64,
}

// ---------------------------------------------------------------- هوية الجهاز

pub fn ensure_node_config(conn: &Connection) -> Result<(String, String), SyncError> {
    let existing: Option<(String, String)> = conn
        .query_row(
            "SELECT node_id, node_name FROM node_config WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some(x) = existing {
        return Ok(x);
    }
    let id = uuid::Uuid::new_v4().to_string();
    let name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "جهاز".to_string());
    conn.execute(
        "INSERT INTO node_config (id, node_id, node_name) VALUES (1, ?1, ?2)",
        params![id, name],
    )?;
    Ok((id, name))
}

pub fn node_id(conn: &Connection) -> Result<String, SyncError> {
    Ok(ensure_node_config(conn)?.0)
}

// ------------------------------------------------------------ مفتاح المزامنة

pub fn save_sync_key(
    conn: &Connection,
    vault_key: &[u8; 32],
    sync_key: &[u8; 32],
) -> Result<(), SyncError> {
    ensure_node_config(conn)?;
    let (ct, nonce) = crypto::encrypt(vault_key, sync_key);
    conn.execute(
        "UPDATE node_config SET sync_key_wrapped = ?1, sync_key_nonce = ?2 WHERE id = 1",
        params![ct, nonce.to_vec()],
    )?;
    Ok(())
}

pub fn load_sync_key(
    conn: &Connection,
    vault_key: &[u8; 32],
) -> Result<Option<[u8; 32]>, SyncError> {
    let row: Option<(Option<Vec<u8>>, Option<Vec<u8>>)> = conn
        .query_row(
            "SELECT sync_key_wrapped, sync_key_nonce FROM node_config WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match row {
        Some((Some(ct), Some(nonce))) => {
            let plain = crypto::decrypt(vault_key, &ct, &nonce).map_err(SyncError::Protocol)?;
            let key: [u8; 32] = plain
                .try_into()
                .map_err(|_| SyncError::Protocol("مفتاح المزامنة المخزَّن تالف".into()))?;
            Ok(Some(key))
        }
        _ => Ok(None),
    }
}

// ------------------------------------------------------------ سجل الأحداث

/// يسجّل حدثًا محليًا (التغيير نفسه طُبِّق أصلًا على الجداول من قِبل الأمر).
pub fn log_local_event(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
    operation: &str,
    payload: &Value,
) -> Result<(), SyncError> {
    let node = node_id(conn)?;
    let seq: i64 = conn.query_row(
        "SELECT COALESCE(MAX(origin_seq), 0) + 1 FROM sync_events WHERE origin_node_id = ?1",
        params![node],
        |r| r.get(0),
    )?;
    let lamport: i64 = conn.query_row(
        "SELECT COALESCE(MAX(lamport), 0) + 1 FROM sync_events",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO sync_events
            (origin_node_id, origin_seq, entity_type, entity_id, operation, payload, lamport, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%s','now'))",
        params![node, seq, entity_type, entity_id, operation, payload.to_string(), lamport],
    )?;
    Ok(())
}

/// متجه الإصدار: أعلى تسلسل متصل معروف لكل جهاز مصدر.
pub fn version_vector(conn: &Connection) -> Result<HashMap<String, i64>, SyncError> {
    let mut stmt = conn
        .prepare("SELECT origin_node_id, MAX(origin_seq) FROM sync_events GROUP BY origin_node_id")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    let mut map = HashMap::new();
    for row in rows {
        let (k, v) = row?;
        map.insert(k, v);
    }
    Ok(map)
}

/// الأحداث التي يفتقدها الطرف الآخر (حسب متجهه)، مرتّبة بالتسلسل لكل مصدر.
pub fn events_missing(
    conn: &Connection,
    peer: &HashMap<String, i64>,
) -> Result<Vec<SyncEvent>, SyncError> {
    let mine = version_vector(conn)?;
    let mut origins: Vec<String> = mine.keys().cloned().collect();
    origins.sort();

    let mut out = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT origin_node_id, origin_seq, entity_type, entity_id, operation, payload, lamport, created_at
         FROM sync_events WHERE origin_node_id = ?1 AND origin_seq > ?2 ORDER BY origin_seq",
    )?;
    for origin in origins {
        let from = peer.get(&origin).copied().unwrap_or(0);
        let rows = stmt.query_map(params![origin, from], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
            ))
        })?;
        for row in rows {
            let (o, seq, et, eid, op, payload, lamport, created_at) = row?;
            out.push(SyncEvent {
                origin_node_id: o,
                origin_seq: seq,
                entity_type: et,
                entity_id: eid,
                operation: op,
                payload: serde_json::from_str(&payload)?,
                lamport,
                created_at,
            });
        }
    }
    Ok(out)
}

// ------------------------------------------------------- تطبيق أحداث مستلَمة

/// يطبّق أحداثًا واردة. يُرجع عدد الأحداث الجديدة فعلًا (المكرَّرة تُتجاهل).
pub fn apply_remote_events(
    conn: &Connection,
    events: &[SyncEvent],
    vault_dir: &Path,
) -> Result<usize, SyncError> {
    // المفاتيح الأجنبية تُعطَّل أثناء التطبيق: الأحداث قد تصل بترتيب لا يحترم
    // العلاقات (وثيقة قبل فئتها مثلًا)، والحالة النهائية تتقارب بعد اكتمال المزامنة.
    conn.pragma_update(None, "foreign_keys", "OFF")?;
    let result: Result<usize, SyncError> = (|| {
        let mut n = 0;
        for ev in events {
            if apply_one(conn, ev, vault_dir)? {
                n += 1;
            }
        }
        Ok(n)
    })();
    conn.pragma_update(None, "foreign_keys", "ON")?;
    result
}

fn apply_one(conn: &Connection, ev: &SyncEvent, vault_dir: &Path) -> Result<bool, SyncError> {
    if !matches!(
        ev.entity_type.as_str(),
        "document" | "category" | "user" | "department" | "document_type" | "disposal_log"
    )
        || !matches!(ev.operation.as_str(), "upsert" | "delete")
    {
        return Err(SyncError::Protocol("نوع حدث غير معروف".into()));
    }

    let tx = conn.unchecked_transaction()?;

    let cur: i64 = tx.query_row(
        "SELECT COALESCE(MAX(origin_seq), 0) FROM sync_events WHERE origin_node_id = ?1",
        params![ev.origin_node_id],
        |r| r.get(0),
    )?;
    if ev.origin_seq <= cur {
        return Ok(false); // مكرَّر
    }
    if ev.origin_seq != cur + 1 {
        return Err(SyncError::Protocol(format!(
            "فجوة في تسلسل أحداث الجهاز {}",
            ev.origin_node_id
        )));
    }

    tx.execute(
        "INSERT INTO sync_events
            (origin_node_id, origin_seq, entity_type, entity_id, operation, payload, lamport, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            ev.origin_node_id,
            ev.origin_seq,
            ev.entity_type,
            ev.entity_id,
            ev.operation,
            ev.payload.to_string(),
            ev.lamport,
            ev.created_at
        ],
    )?;

    // Last-Writer-Wins: الحدث الأحدث (lamport ثم معرّف الجهاز) هو من يحدد حالة الكيان.
    // بما أن كل حدث يحمل الحالة كاملة، فالنتيجة النهائية واحدة على كل الأجهزة
    // مهما كان ترتيب وصول الأحداث.
    let latest: (String, i64) = tx.query_row(
        "SELECT origin_node_id, origin_seq FROM sync_events
         WHERE entity_type = ?1 AND entity_id = ?2
         ORDER BY lamport DESC, origin_node_id DESC, origin_seq DESC LIMIT 1",
        params![ev.entity_type, ev.entity_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if latest.0 == ev.origin_node_id && latest.1 == ev.origin_seq {
        apply_state(&tx, ev, vault_dir)?;
    }

    tx.commit()?;
    Ok(true)
}

fn str_field(p: &Value, k: &str) -> Result<String, SyncError> {
    p.get(k)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| SyncError::Protocol(format!("حقل ناقص في الحدث: {k}")))
}
fn opt_str(p: &Value, k: &str) -> Option<String> {
    p.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}
fn int_field(p: &Value, k: &str) -> i64 {
    p.get(k).and_then(|v| v.as_i64()).unwrap_or(0)
}

fn apply_state(tx: &Connection, ev: &SyncEvent, vault_dir: &Path) -> Result<(), SyncError> {
    let p = &ev.payload;
    let id = &ev.entity_id;
    match (ev.entity_type.as_str(), ev.operation.as_str()) {
        ("category", "upsert") => {
            tx.execute(
                "INSERT INTO categories (id, name, parent_id, created_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name, parent_id = excluded.parent_id",
                params![id, str_field(p, "name")?, opt_str(p, "parent_id"), int_field(p, "created_at")],
            )?;
        }
        ("category", "delete") => {
            tx.execute("UPDATE documents SET category_id = NULL WHERE category_id = ?1", params![id])?;
            tx.execute("UPDATE categories SET parent_id = NULL WHERE parent_id = ?1", params![id])?;
            tx.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
        }
        ("document", "upsert") => {
            let hash = str_field(p, "file_hash")?;
            let path = blob_path(vault_dir, &hash)?;
            tx.execute(
                "INSERT INTO documents
                    (id, title, category_id, file_hash, file_path, file_size, mime_type, ocr_text,
                     department_id, document_type_id, registry_number, confidentiality_level, status, metadata,
                     legal_hold, disposed_at, physical_location, physical_status, borrowed_by, borrowed_at,
                     created_by, created_at, updated_at, origin_node_id, is_deleted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)
                 ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title, category_id = excluded.category_id,
                    ocr_text = excluded.ocr_text,
                    department_id = excluded.department_id, document_type_id = excluded.document_type_id,
                    confidentiality_level = excluded.confidentiality_level, status = excluded.status,
                    metadata = excluded.metadata,
                    legal_hold = excluded.legal_hold, disposed_at = excluded.disposed_at,
                    physical_location = excluded.physical_location, physical_status = excluded.physical_status,
                    borrowed_by = excluded.borrowed_by, borrowed_at = excluded.borrowed_at,
                    updated_at = excluded.updated_at, is_deleted = excluded.is_deleted",
                params![
                    id,
                    str_field(p, "title")?,
                    opt_str(p, "category_id"),
                    hash,
                    path.to_string_lossy().to_string(),
                    int_field(p, "file_size"),
                    opt_str(p, "mime_type"),
                    opt_str(p, "ocr_text"),
                    opt_str(p, "department_id"),
                    opt_str(p, "document_type_id"),
                    opt_str(p, "registry_number"),
                    p.get("confidentiality_level").and_then(|v| v.as_i64()).unwrap_or(1),
                    p.get("status").and_then(|v| v.as_str()).unwrap_or("approved").to_string(),
                    opt_str(p, "metadata"),
                    p.get("legal_hold").and_then(|v| v.as_i64()).unwrap_or(0),
                    p.get("disposed_at").and_then(|v| v.as_i64()),
                    opt_str(p, "physical_location"),
                    p.get("physical_status").and_then(|v| v.as_str()).unwrap_or("none").to_string(),
                    opt_str(p, "borrowed_by"),
                    p.get("borrowed_at").and_then(|v| v.as_i64()),
                    opt_str(p, "created_by"),
                    int_field(p, "created_at"),
                    p.get("updated_at").and_then(|v| v.as_i64()),
                    opt_str(p, "origin_node_id").unwrap_or_else(|| ev.origin_node_id.clone()),
                    int_field(p, "is_deleted"),
                ],
            )?;

            // إتلاف وصل من جهاز آخر: احذف نسخة هذا الجهاز من الملف أيضًا (إن وصلته أصلًا)،
            // بنفس شرط عدم وجود وثيقة حيّة أخرى تشير لنفس المحتوى
            if p.get("disposed_at").and_then(|v| v.as_i64()).is_some() {
                let other_refs: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM documents WHERE file_hash = ?1 AND id <> ?2 AND is_deleted = 0",
                    params![hash, id], |r| r.get(0),
                )?;
                if other_refs == 0 {
                    let _ = std::fs::remove_file(&path); // best-effort — طبيعي ألا يكون الملف وصل لهذا الجهاز أصلًا
                }
            }
        }
        ("disposal_log", "upsert") => {
            tx.execute(
                "INSERT INTO disposal_log (id, document_id, title, registry_number, document_type_id, disposed_by, disposed_at, reason, node_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    id,
                    str_field(p, "document_id")?,
                    str_field(p, "title")?,
                    opt_str(p, "registry_number"),
                    opt_str(p, "document_type_id"),
                    opt_str(p, "disposed_by"),
                    int_field(p, "disposed_at"),
                    opt_str(p, "reason"),
                    opt_str(p, "node_id").unwrap_or_else(|| ev.origin_node_id.clone()),
                ],
            )?;
        }
        ("department", "upsert") => {
            tx.execute(
                "INSERT INTO departments (id, name_ar, name_fr, code, parent_id, head_user_id, is_active, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                    name_ar = excluded.name_ar, name_fr = excluded.name_fr, code = excluded.code,
                    parent_id = excluded.parent_id, head_user_id = excluded.head_user_id,
                    is_active = excluded.is_active",
                params![
                    id,
                    str_field(p, "name_ar")?,
                    opt_str(p, "name_fr"),
                    opt_str(p, "code"),
                    opt_str(p, "parent_id"),
                    opt_str(p, "head_user_id"),
                    p.get("is_active").and_then(|v| v.as_i64()).unwrap_or(1),
                    int_field(p, "created_at"),
                ],
            )?;
        }
        ("department", "delete") => {
            tx.execute("UPDATE departments SET is_active = 0 WHERE id = ?1", params![id])?;
        }
        ("document_type", "upsert") => {
            tx.execute(
                "INSERT INTO document_types (id, name, created_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name",
                params![id, str_field(p, "name")?, int_field(p, "created_at")],
            )?;
        }
        ("document_type", "delete") => {
            tx.execute("DELETE FROM document_types WHERE id = ?1", params![id])?;
        }
        ("document", "delete") => {
            tx.execute(
                "UPDATE documents SET is_deleted = 1, updated_at = ?1 WHERE id = ?2",
                params![p.get("updated_at").and_then(|v| v.as_i64()), id],
            )?;
        }
        ("user", "upsert") => {
            let username = resolve_username(tx, id, &str_field(p, "username")?)?;
            tx.execute(
                "INSERT INTO users
                    (id, username, password_hash, role, full_name, department_id, clearance_level, created_at, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    username = excluded.username, password_hash = excluded.password_hash,
                    role = excluded.role, full_name = excluded.full_name,
                    department_id = excluded.department_id, clearance_level = excluded.clearance_level,
                    is_active = excluded.is_active",
                params![
                    id,
                    username,
                    str_field(p, "password_hash")?,
                    str_field(p, "role")?,
                    opt_str(p, "full_name"),
                    opt_str(p, "department_id"),
                    p.get("clearance_level").and_then(|v| v.as_i64()).unwrap_or(1),
                    int_field(p, "created_at"),
                    p.get("is_active").and_then(|v| v.as_i64()).unwrap_or(1),
                ],
            )?;
        }
        ("user", "delete") => {
            tx.execute("UPDATE users SET is_active = 0 WHERE id = ?1", params![id])?;
        }
        _ => {}
    }
    Ok(())
}

/// اسم المستخدم فريد. إذا أنشأ جهازان مستخدمين مختلفين بنفس الاسم، فالقاعدة حتمية
/// على كل الأجهزة: صاحب المعرّف الأصغر يحتفظ بالاسم، والآخر يُلحَق به لاحقة من معرّفه.
fn resolve_username(tx: &Connection, id: &str, username: &str) -> Result<String, SyncError> {
    let other: Option<String> = tx
        .query_row(
            "SELECT id FROM users WHERE username = ?1 AND id <> ?2",
            params![username, id],
            |r| r.get(0),
        )
        .optional()?;
    match other {
        None => Ok(username.to_string()),
        Some(other_id) => {
            if other_id.as_str() < id {
                Ok(format!("{}-{}", username, &id[..id.len().min(4)]))
            } else {
                tx.execute(
                    "UPDATE users SET username = ?1 WHERE id = ?2",
                    params![format!("{}-{}", username, &other_id[..other_id.len().min(4)]), other_id],
                )?;
                Ok(username.to_string())
            }
        }
    }
}

// ------------------------------------------------------------------ الملفات

pub fn is_valid_hash(h: &str) -> bool {
    h.len() == 64 && h.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// مسار الملف في الخزنة — نفس تخطيط add_document. الـ hash يأتي من الشبكة أحيانًا،
/// لذلك يُتحقق منه بصرامة لمنع أي محاولة تجاوز مسارات (Path Traversal).
pub fn blob_path(vault_dir: &Path, hash: &str) -> Result<PathBuf, SyncError> {
    if !is_valid_hash(hash) {
        return Err(SyncError::Protocol("قيمة hash غير صالحة".into()));
    }
    Ok(vault_dir.join(&hash[0..2]).join(hash))
}

/// الوثائق (غير المحذوفة) التي لا يوجد ملفها على هذا الجهاز بعد.
pub fn missing_files(conn: &Connection, vault_dir: &Path) -> Result<Vec<String>, SyncError> {
    let mut stmt =
        conn.prepare("SELECT DISTINCT file_hash FROM documents WHERE is_deleted = 0")?;
    let hashes = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for h in hashes {
        let h = h?;
        if let Ok(p) = blob_path(vault_dir, &h) {
            if !p.exists() {
                out.push(h);
            }
        }
    }
    Ok(out)
}

/// يقرأ ملفًا من الخزنة ويفك تشفيره (صيغة التخزين: [Nonce 12][Ciphertext]).
pub fn read_blob_plain(vault_dir: &Path, key: &[u8; 32], hash: &str) -> Result<Vec<u8>, SyncError> {
    let raw = std::fs::read(blob_path(vault_dir, hash)?)?;
    if raw.len() < crypto::NONCE_LEN {
        return Err(SyncError::Protocol("ملف تالف".into()));
    }
    let (nonce, ct) = raw.split_at(crypto::NONCE_LEN);
    crypto::decrypt(key, ct, nonce).map_err(SyncError::Protocol)
}

/// يتحقق أن SHA-256 للمحتوى يطابق الـ hash المتوقع، ثم يشفّره بمفتاح هذا الجهاز ويخزّنه.
pub fn write_blob_from_plain(
    vault_dir: &Path,
    key: &[u8; 32],
    hash: &str,
    plain: &[u8],
) -> Result<(), SyncError> {
    let path = blob_path(vault_dir, hash)?;
    let mut h = Sha256::new();
    h.update(plain);
    if format!("{:x}", h.finalize()) != hash {
        return Err(SyncError::Protocol("الملف المستلَم لا يطابق بصمته (SHA-256)".into()));
    }
    if path.exists() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let (ct, nonce) = crypto::encrypt(key, plain);
    let mut out = Vec::with_capacity(nonce.len() + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    std::fs::write(path, out)?;
    Ok(())
}
