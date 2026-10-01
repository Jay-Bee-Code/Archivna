-- ============================================================
-- Migration 0002: الهيكل التنظيمي والترقيم الإداري
-- ============================================================

-- الأقسام (شجرية عبر parent_id — قائمة مسطّحة في الواجهة حاليًا، قابلة لعرض شجري لاحقًا)
CREATE TABLE IF NOT EXISTS departments (
    id TEXT PRIMARY KEY,
    name_ar TEXT NOT NULL,
    name_fr TEXT,
    code TEXT,                          -- رمز مختصر مثل DRH، DAG
    parent_id TEXT REFERENCES departments(id),
    head_user_id TEXT REFERENCES users(id),
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);

-- أنواع الوثائق (الشكل الإداري: مراسلة، قرار، عقد...) — مختلف عن الفئة (الموضوع)
CREATE TABLE IF NOT EXISTS document_types (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- عدّاد رقم القيد: مستقل لكل (جهاز، قسم، سنة) — يمنع التصادم بين الأجهزة
-- غير المتصلة دون أي تنسيق مسبق (راجع تعليق الترقيم في commands/documents.rs)
CREATE TABLE IF NOT EXISTS registry_counters (
    node_id TEXT NOT NULL,
    department_id TEXT NOT NULL,
    year INTEGER NOT NULL,
    next_seq INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (node_id, department_id, year)
);

-- تصريح الاطّلاع (Clearance) — مستقل عن الدور (role): الدور يحكم نوع العملية
-- (قراءة/كتابة)، والتصريح يحكم أعلى مستوى سرية يمكن للمستخدم رؤيته
ALTER TABLE users ADD COLUMN clearance_level INTEGER NOT NULL DEFAULT 1;
ALTER TABLE users ADD COLUMN department_id TEXT REFERENCES departments(id);

ALTER TABLE documents ADD COLUMN department_id TEXT REFERENCES departments(id);
ALTER TABLE documents ADD COLUMN document_type_id TEXT REFERENCES document_types(id);
ALTER TABLE documents ADD COLUMN registry_number TEXT;
-- 1=عادي 2=محدود التداول 3=سري 4=سري جدًا
ALTER TABLE documents ADD COLUMN confidentiality_level INTEGER NOT NULL DEFAULT 1;
-- draft|in_review|approved|archived|superseded
ALTER TABLE documents ADD COLUMN status TEXT NOT NULL DEFAULT 'approved';
-- حقول وصفية إضافية خاصة بنوع الوثيقة (JSON حر — انظر القسم 2 من الاقتراح)
ALTER TABLE documents ADD COLUMN metadata TEXT;

CREATE INDEX IF NOT EXISTS idx_documents_department ON documents(department_id);
CREATE INDEX IF NOT EXISTS idx_documents_registry ON documents(registry_number);
CREATE INDEX IF NOT EXISTS idx_documents_status ON documents(status);
