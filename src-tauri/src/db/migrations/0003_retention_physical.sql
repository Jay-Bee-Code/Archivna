-- ============================================================
-- Migration 0003: الاحتفاظ والإتلاف + الأرشيف الورقي
-- ============================================================

-- مدة الاحتفاظ بالسنوات لكل نوع وثيقة — NULL يعني "دائم، لا إتلاف تلقائي أبدًا"
ALTER TABLE document_types ADD COLUMN retention_years INTEGER;

-- تجميد قانوني: يمنع الإتلاف حتى لو انتهت مدة الاحتفاظ (قضية، تحقيق...)
ALTER TABLE documents ADD COLUMN legal_hold INTEGER NOT NULL DEFAULT 0;
-- NULL = لم تُتلف بعد؛ قيمة = وقت الإتلاف الفعلي
ALTER TABLE documents ADD COLUMN disposed_at INTEGER;

-- محضر الإتلاف: سجل دائم لا يُحذف أبدًا (حتى بعد إتلاف الوثيقة نفسها)،
-- يُزامَن كسجل Append-Only عبر كل الأجهزة — إثبات إداري/قانوني لما أُتلف ومتى ولماذا
CREATE TABLE IF NOT EXISTS disposal_log (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    title TEXT NOT NULL,
    registry_number TEXT,
    document_type_id TEXT,
    disposed_by TEXT,
    disposed_at INTEGER NOT NULL,
    reason TEXT,
    node_id TEXT NOT NULL
);

-- الأرشيف الورقي: ربط النسخة الرقمية بموقع أصلها الورقي وحالته
ALTER TABLE documents ADD COLUMN physical_location TEXT;     -- "مبنى أ / طابق 2 / خزانة 4 / رف 3 / صندوق 12"
-- none=لا نسخة ورقية مسجّلة | present=موجود | borrowed=معار | missing=مفقود | destroyed=متلف ماديًا
ALTER TABLE documents ADD COLUMN physical_status TEXT NOT NULL DEFAULT 'none';
ALTER TABLE documents ADD COLUMN borrowed_by TEXT REFERENCES users(id);
ALTER TABLE documents ADD COLUMN borrowed_at INTEGER;

CREATE INDEX IF NOT EXISTS idx_disposal_log_document ON disposal_log(document_id);
CREATE INDEX IF NOT EXISTS idx_documents_physical_status ON documents(physical_status);
