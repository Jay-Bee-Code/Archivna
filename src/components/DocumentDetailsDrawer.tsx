import { useEffect, useState } from "react";
import {
  api,
  Document,
  Role,
  Correspondent,
  CORRESPONDENT_KIND_LABELS,
  CONFIDENTIALITY_LABELS,
  STATUS_LABELS,
  PHYSICAL_STATUS_LABELS,
  errorMessage,
} from "../lib/api";
import { IconClose, IconBox, IconScale } from "./icons/Icon";

/** الحالات التي يختارها المراجع يدويًا — "disposed" تُضبط حصرًا عبر أمر الإتلاف */
const SELECTABLE_STATUSES = ["draft", "in_review", "approved", "archived", "superseded"];

function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString("ar-DZ", { year: "numeric", month: "long", day: "numeric" });
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} بايت`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} كيلوبايت`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} ميغابايت`;
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4 py-2 border-b border-border-light/60 dark:border-white/5 last:border-0">
      <dt className="text-xs text-text-secondary dark:text-white/40 shrink-0">{label}</dt>
      <dd className="text-sm text-text-primary dark:text-white text-end min-w-0 break-words">{children}</dd>
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-xs font-semibold text-text-secondary dark:text-white/40 uppercase tracking-wide mb-2">
      {children}
    </h3>
  );
}

export default function DocumentDetailsDrawer({
  doc,
  role,
  categoryName,
  departmentName,
  typeName,
  correspondent,
  onClose,
  onChanged,
}: {
  doc: Document;
  role: Role;
  categoryName: string | null;
  departmentName: string | null;
  typeName: string | null;
  correspondent: Correspondent | null;
  onClose: () => void;
  onChanged: () => void;
}) {
  // المراجع والمدير يغيّران الحالة؛ الأرشيفي والمدير يديران الأصل الورقي
  const canReview = role === "admin" || role === "reviewer";
  const canEditPhysical = role === "admin" || role === "archivist";

  const [status, setStatus] = useState(doc.status);
  const [physicalStatus, setPhysicalStatus] = useState(doc.physical_status);
  const [location, setLocation] = useState(doc.physical_location ?? "");
  const [borrowerName, setBorrowerName] = useState<string | null>(null);

  const [savingStatus, setSavingStatus] = useState(false);
  const [savingPhysical, setSavingPhysical] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  // اسم المستعير: list_users متاح للمدير والأرشيفي فقط، فيفشل بصمت لغيرهما
  useEffect(() => {
    if (doc.physical_status !== "borrowed" || !doc.borrowed_by) {
      setBorrowerName(null);
      return;
    }
    api
      .listUsers()
      .then((users) => {
        const u = users.find((x) => x.id === doc.borrowed_by);
        setBorrowerName(u ? u.full_name || u.username : null);
      })
      .catch(() => setBorrowerName(null));
  }, [doc.physical_status, doc.borrowed_by]);

  async function saveStatus() {
    setSavingStatus(true);
    setError(null);
    setMessage(null);
    try {
      await api.updateDocumentStatus(doc.id, status);
      setMessage("تم تحديث حالة الوثيقة");
      onChanged();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSavingStatus(false);
    }
  }

  async function savePhysical() {
    setSavingPhysical(true);
    setError(null);
    setMessage(null);
    try {
      await api.updatePhysical(doc.id, location.trim() || null, physicalStatus);
      setMessage("تم تحديث بيانات الأصل الورقي");
      onChanged();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSavingPhysical(false);
    }
  }

  const fieldClass =
    "w-full px-3 py-2 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white text-sm focus:outline-none focus:ring-2 focus:ring-accent";
  const btnClass =
    "shrink-0 px-4 py-2 rounded-lg bg-primary text-white text-sm font-medium hover:bg-accent disabled:opacity-40 transition-colors";

  const physicalChanged =
    physicalStatus !== doc.physical_status || (location.trim() || null) !== (doc.physical_location ?? null);

  return (
    <div className="fixed inset-0 z-40 flex" onClick={onClose}>
      <div className="flex-1 bg-black/40" />
      <aside
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-md h-full bg-white dark:bg-[#132033] border-s border-border-light dark:border-white/10 shadow-2xl overflow-y-auto flex flex-col"
      >
        <div className="flex items-start justify-between gap-3 p-5 border-b border-border-light dark:border-white/10">
          <div className="min-w-0">
            <h2 className="text-base font-bold text-primary dark:text-white break-words">{doc.title}</h2>
            {doc.registry_number && (
              <p className="text-xs font-mono text-text-secondary dark:text-white/40 mt-1">{doc.registry_number}</p>
            )}
          </div>
          <button
            onClick={onClose}
            className="shrink-0 text-text-secondary dark:text-white/50 hover:text-danger hover:bg-danger/5 p-1.5 rounded-lg transition-colors"
          >
            <IconClose size={16} />
          </button>
        </div>

        <div className="p-5 flex flex-col gap-6">
          <section>
            <SectionTitle>البيانات</SectionTitle>
            <dl>
              <Row label="نوع الوثيقة">{typeName ?? "—"}</Row>
              <Row label="القسم">{departmentName ?? "بلا قسم (مركزي)"}</Row>
              <Row label="الفئة">{categoryName ?? "—"}</Row>
              <Row label="جهة المراسلة">
                {correspondent
                  ? `${correspondent.name} (${CORRESPONDENT_KIND_LABELS[correspondent.kind] || correspondent.kind})`
                  : "—"}
              </Row>
              <Row label="مستوى السرية">{CONFIDENTIALITY_LABELS[doc.confidentiality_level]}</Row>
              <Row label="الحالة">{STATUS_LABELS[doc.status] || doc.status}</Row>
              <Row label="تاريخ الإضافة">{formatDate(doc.created_at)}</Row>
              <Row label="الحجم">{formatSize(doc.file_size)}</Row>
              <Row label="النوع التقني">{doc.mime_type ?? "—"}</Row>
              <Row label="نص مستخرَج (OCR)">{doc.has_ocr ? "نعم" : "لا"}</Row>
              {doc.legal_hold && (
                <Row label="التجميد القانوني">
                  <span className="inline-flex items-center gap-1 text-warning">
                    <IconScale size={13} /> مجمَّدة — لا يمكن إتلافها
                  </span>
                </Row>
              )}
            </dl>
          </section>

          {canReview && (
            <section>
              <SectionTitle>مراجعة الوثيقة</SectionTitle>
              <div className="flex gap-2">
                <select value={status} onChange={(e) => setStatus(e.target.value)} className={fieldClass}>
                  {SELECTABLE_STATUSES.map((s) => (
                    <option key={s} value={s}>{STATUS_LABELS[s]}</option>
                  ))}
                </select>
                <button onClick={saveStatus} disabled={savingStatus || status === doc.status} className={btnClass}>
                  {savingStatus ? "…" : "تطبيق"}
                </button>
              </div>
            </section>
          )}

          <section>
            <SectionTitle>
              <span className="inline-flex items-center gap-1.5">
                <IconBox size={13} /> الأصل الورقي
              </span>
            </SectionTitle>

            {canEditPhysical ? (
              <div className="flex flex-col gap-2.5">
                <select value={physicalStatus} onChange={(e) => setPhysicalStatus(e.target.value)} className={fieldClass}>
                  {Object.entries(PHYSICAL_STATUS_LABELS).map(([k, label]) => (
                    <option key={k} value={k}>{label}</option>
                  ))}
                </select>
                <input
                  value={location}
                  onChange={(e) => setLocation(e.target.value)}
                  placeholder="الموقع: مبنى / طابق / خزانة / رف / صندوق"
                  className={fieldClass}
                />
                {physicalStatus === "borrowed" && (
                  <p className="text-[11px] text-text-secondary dark:text-white/40">
                    عند الحفظ بحالة "معار" يُسجَّل المستعير تلقائيًا باسمك، مع وقت الإعارة.
                  </p>
                )}
                <button onClick={savePhysical} disabled={savingPhysical || !physicalChanged} className={btnClass}>
                  {savingPhysical ? "جارٍ الحفظ…" : "حفظ بيانات الأصل"}
                </button>
              </div>
            ) : (
              <dl>
                <Row label="الحالة">{PHYSICAL_STATUS_LABELS[doc.physical_status] || doc.physical_status}</Row>
                <Row label="الموقع">{doc.physical_location ?? "—"}</Row>
              </dl>
            )}

            {doc.physical_status === "borrowed" && (
              <p className="text-xs text-warning mt-2.5">
                معار{borrowerName ? ` إلى ${borrowerName}` : ""}
                {doc.borrowed_at ? ` منذ ${formatDate(doc.borrowed_at)}` : ""}
              </p>
            )}
          </section>

          {error && <p className="text-sm text-danger bg-danger/10 rounded-md p-3">{error}</p>}
          {message && <p className="text-sm text-success bg-success/10 rounded-md p-3">{message}</p>}
        </div>
      </aside>
    </div>
  );
}
