import { useEffect, useState } from "react";
import { api, Department, DocumentType, errorMessage } from "../lib/api";

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex-1 min-w-0">
      <h3 className="text-xs font-semibold text-text-secondary dark:text-white/40 uppercase tracking-wide mb-2">
        {title}
      </h3>
      <div className="flex flex-col gap-1.5">{children}</div>
    </div>
  );
}

export default function OrgSettingsPanel({ onClose }: { onClose: () => void }) {
  const [departments, setDepartments] = useState<Department[]>([]);
  const [types, setTypes] = useState<DocumentType[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [newDeptName, setNewDeptName] = useState("");
  const [newDeptCode, setNewDeptCode] = useState("");
  const [newTypeName, setNewTypeName] = useState("");

  function refresh() {
    api.listDepartments().then(setDepartments).catch(() => {});
    api.listDocumentTypes().then(setTypes).catch(() => {});
  }
  useEffect(refresh, []);

  async function handleAddDept(e: React.FormEvent) {
    e.preventDefault();
    if (!newDeptName.trim()) return;
    try {
      await api.addDepartment({
        name_ar: newDeptName.trim(),
        name_fr: null,
        code: newDeptCode.trim() || null,
        parent_id: null,
      });
      setNewDeptName("");
      setNewDeptCode("");
      refresh();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleDeleteDept(id: string) {
    try {
      await api.deleteDepartment(id);
      refresh();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleAddType(e: React.FormEvent) {
    e.preventDefault();
    if (!newTypeName.trim()) return;
    try {
      await api.addDocumentType(newTypeName.trim());
      setNewTypeName("");
      refresh();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleDeleteType(id: string) {
    try {
      await api.deleteDocumentType(id);
      refresh();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  const rowClass =
    "flex items-center justify-between gap-2 text-sm px-3 py-2 rounded-lg bg-black/[0.02] dark:bg-white/[0.03]";
  const inputClass =
    "flex-1 min-w-0 text-sm px-3 py-2 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent";

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-40 px-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-2xl bg-white dark:bg-[#132033] border border-border-light dark:border-white/10 rounded-2xl p-6 flex flex-col gap-4 max-h-[85vh] overflow-y-auto"
      >
        <div className="flex items-center justify-between">
          <h2 className="text-base font-bold text-primary dark:text-white">الهيكل التنظيمي وأنواع الوثائق</h2>
          <button onClick={onClose} className="text-text-secondary dark:text-white/50 hover:text-danger text-sm">
            إغلاق ✕
          </button>
        </div>

        {error && <p className="text-sm text-danger bg-danger/10 rounded-md p-2">{error}</p>}

        <div className="flex flex-col sm:flex-row gap-6">
          <Section title="الأقسام">
            {departments.length === 0 && (
              <p className="text-xs text-text-secondary dark:text-white/40">لا أقسام بعد.</p>
            )}
            {departments.map((d) => (
              <div key={d.id} className={rowClass}>
                <span className="truncate text-text-primary dark:text-white">
                  {d.name_ar} {d.code && <span className="text-text-secondary dark:text-white/40">({d.code})</span>}
                </span>
                <button onClick={() => handleDeleteDept(d.id)} className="text-danger/70 hover:text-danger text-xs shrink-0">
                  حذف
                </button>
              </div>
            ))}
            <form onSubmit={handleAddDept} className="flex gap-1.5 mt-1">
              <input value={newDeptName} onChange={(e) => setNewDeptName(e.target.value)} placeholder="اسم القسم" className={inputClass} />
              <input value={newDeptCode} onChange={(e) => setNewDeptCode(e.target.value)} placeholder="الرمز" className="w-20 text-sm px-2 py-2 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent" />
              <button type="submit" className="shrink-0 text-xs px-3 rounded-lg bg-accent text-white hover:bg-primary">+</button>
            </form>
          </Section>

          <Section title="أنواع الوثائق">
            {types.length === 0 && (
              <p className="text-xs text-text-secondary dark:text-white/40">لا أنواع بعد.</p>
            )}
            {types.map((t) => (
              <div key={t.id} className={rowClass}>
                <span className="truncate text-text-primary dark:text-white">{t.name}</span>
                <button onClick={() => handleDeleteType(t.id)} className="text-danger/70 hover:text-danger text-xs shrink-0">
                  حذف
                </button>
              </div>
            ))}
            <form onSubmit={handleAddType} className="flex gap-1.5 mt-1">
              <input value={newTypeName} onChange={(e) => setNewTypeName(e.target.value)} placeholder="اسم النوع" className={inputClass} />
              <button type="submit" className="shrink-0 text-xs px-3 rounded-lg bg-accent text-white hover:bg-primary">+</button>
            </form>
          </Section>
        </div>
      </div>
    </div>
  );
}
