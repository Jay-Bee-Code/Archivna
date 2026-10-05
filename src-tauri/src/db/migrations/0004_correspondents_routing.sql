-- ============================================================
-- Migration 0004: جهات المراسلة الخارجية + سجل الإحالة والتوزيع
-- ============================================================

-- جهات خارجية (وزارات، ولايات، شركات، أفراد) — تُختار من قائمة بدل كتابة حرة
-- متكرّرة، فيتوحّد اسم الجهة عبر كل الوثائق المرتبطة بها
CREATE TABLE IF NOT EXISTS correspondents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'other', -- ministry|wilaya|company|individual|other
    address TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);

-- الجهة المرتبطة بالوثيقة: المُرسِل في الوارد، أو المُستقبِل في الصادر
ALTER TABLE documents ADD COLUMN correspondent_id TEXT REFERENCES correspondents(id);

-- سجل الإحالة والتوزيع الداخلي: لمن أُحيلت نسخة من الوثيقة داخل المؤسسة ومتى.
-- سجل تراكمي (Append-Only) بتصميم — لا تعديل ولا حذف لقيد إحالة بعد تسجيله،
-- فهو إثبات إداري لمسار الوثيقة (نفس فلسفة disposal_log)
CREATE TABLE IF NOT EXISTS document_routing (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    routed_to_user_id TEXT NOT NULL REFERENCES users(id),
    routed_by TEXT REFERENCES users(id),
    routed_at INTEGER NOT NULL,
    note TEXT
);

CREATE INDEX IF NOT EXISTS idx_documents_correspondent ON documents(correspondent_id);
CREATE INDEX IF NOT EXISTS idx_routing_document ON document_routing(document_id);
CREATE INDEX IF NOT EXISTS idx_routing_user ON document_routing(routed_to_user_id);
