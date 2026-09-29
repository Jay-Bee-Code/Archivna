-- ============================================================
-- مخطط قاعدة البيانات — نظام الأرشفة الحكومي
-- ملاحظة: تُفتح القاعدة عبر SQLCipher (PRAGMA key) قبل تنفيذ هذا الملف —
-- راجع src-tauri/src/db/mod.rs::open_encrypted_db
-- ============================================================

-- إعدادات الخزنة: مفتاح تشفير الملفات (Vault Master Key) مُغلَّف
-- بمفتاح مُشتق عبر Argon2id من نفس العبارة السرية لقاعدة البيانات
CREATE TABLE IF NOT EXISTS vault_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    wrapped_key BLOB NOT NULL,
    wrap_salt BLOB NOT NULL,
    wrap_nonce BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'archivist', -- admin | archivist | reviewer | viewer
    full_name TEXT,
    department TEXT,
    created_at INTEGER NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id TEXT REFERENCES categories(id),
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    category_id TEXT REFERENCES categories(id),
    file_hash TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    mime_type TEXT,
    ocr_text TEXT,
    signature TEXT,
    created_by TEXT REFERENCES users(id),
    created_at INTEGER NOT NULL,
    updated_at INTEGER,
    origin_node_id TEXT NOT NULL DEFAULT 'local',
    is_deleted INTEGER NOT NULL DEFAULT 0
);

CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
    title,
    ocr_text,
    content='documents',
    content_rowid='rowid'
);

-- مزامنة الفهرس FTS تلقائيًا مع جدول الوثائق
CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
    INSERT INTO documents_fts(rowid, title, ocr_text)
    VALUES (new.rowid, new.title, new.ocr_text);
END;

CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
    INSERT INTO documents_fts(documents_fts, rowid, title, ocr_text)
    VALUES ('delete', old.rowid, old.title, old.ocr_text);
END;

CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
    INSERT INTO documents_fts(documents_fts, rowid, title, ocr_text)
    VALUES ('delete', old.rowid, old.title, old.ocr_text);
    INSERT INTO documents_fts(rowid, title, ocr_text)
    VALUES (new.rowid, new.title, new.ocr_text);
END;

CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    document_id TEXT,
    user_id TEXT,
    action TEXT NOT NULL, -- created | viewed | updated | deleted | synced
    timestamp INTEGER NOT NULL,
    node_id TEXT NOT NULL DEFAULT 'local',
    metadata TEXT
);

CREATE INDEX IF NOT EXISTS idx_documents_category ON documents(category_id);
CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_document ON audit_log(document_id);


-- ============================================================
-- Phase 4: المزامنة
-- ============================================================

-- هوية هذا الجهاز (Node) + مفتاح مزامنة المؤسسة مُغلَّفًا بمفتاح الخزنة
CREATE TABLE IF NOT EXISTS node_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    node_id TEXT NOT NULL,
    node_name TEXT NOT NULL,
    sync_key_wrapped BLOB,
    sync_key_nonce BLOB
);

-- سجل الأحداث: كل تغيير (محلي أو مستلَم) يُسجَّل هنا.
-- (origin_node_id, origin_seq) تسلسل متصل لكل جهاز مصدر؛ lamport لترتيب التعارضات.
CREATE TABLE IF NOT EXISTS sync_events (
    origin_node_id TEXT NOT NULL,
    origin_seq INTEGER NOT NULL,
    entity_type TEXT NOT NULL,   -- document | category | user
    entity_id TEXT NOT NULL,
    operation TEXT NOT NULL,     -- upsert | delete
    payload TEXT NOT NULL,       -- JSON
    lamport INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (origin_node_id, origin_seq)
);

CREATE INDEX IF NOT EXISTS idx_sync_entity ON sync_events(entity_type, entity_id, lamport);
