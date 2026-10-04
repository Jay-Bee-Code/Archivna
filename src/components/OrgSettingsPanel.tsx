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

interface DeptNode extends Department {
  children: DeptNode[];
}

/** يبني شجرة من القائمة المسطّحة عبر parent_id — الجذور أولًا، ثم أبناء كل قسم */
function buildTree(flat: Department[]): DeptNode[] {
  const byId = new Map<string, DeptNode>(flat.map((d) => [d.id, { ...d, children: [] }]));
  const roots: DeptNode[] = [];
  for (const node of byId.values()) {
    if (node.parent_id && byId.has(node.parent_id)) {
      byId.get(node.parent_id)!.children.push(node);
    } else {
      roots.push(node);
    }
  }
  return roots;
}

function DeptRow({
  node,
  depth,
  onDelete,
  onAddChild,
}: {
  node: DeptNode;
  depth: number;
  onDelete: (id: string) => void;
  onAddChild: (parentId: string) => void;
}) {
  return (
    <>
      <div
        className="flex items-center justify-between gap-2 text-sm px-3 py-2 rounded-lg bg-black/[0.02] dark:bg-white/[0.03]"
        style={{ marginInlineStart: depth * 18 }}
      >
        <span className="truncate text-text-primary dark:text-white flex items-center gap-1.5">
          {depth > 0 && <span className="text-text-secondary dark:text-white/30">└</span>}
          {node.name_ar}
          {node.code && <span className="text-text-secondary dark:text-white/40 text-xs">({node.code})</span>}
        </span>
        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={() => onAddChild(node.id)}
            className="text-accent hover:text-primary text-xs"
            title="إضافة قسم فرعي"
          >
            + فرعي
          </button>
          <button onClick={() => onDelete(node.id)} className="text-danger/70 hover:text-danger text-xs">
            حذف
          </button>
        </div>
      </div>
      {node.children.map((c) => (
        <DeptRow key={c.id} node={c} depth={depth + 1} onDelete={onDelete} onAddChild={onAddChild} />
      ))}
    </>
  );
}

export default function OrgSettingsPanel({ onClose }: { onClose: () => void }) {
  const [departments, setDepartments] = useState<Department[]>([]);
  const [types, setTypes] = useState<DocumentType[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [newDeptName, setNewDeptName] = useState("");
  const [newDeptCode, setNewDeptCode] = useState("");
  const [newDeptParent, setNewDeptParent] = useState<string | null>(null);
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
        parent_id: newDeptParent,
      });
      setNewDeptName("");
      setNewDeptCode("");
      setNewDeptParent(null);
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

  async function handleSetRetention(id: string, value: string) {
    const years = value.trim() === "" ? null : Number(value);
    try {
      await api.setDocumentTypeRetention(id, years);
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
            {buildTree(departments).map((node) => (
              <DeptRow
                key={node.id}
                node={node}
                depth={0}
                onDelete={handleDeleteDept}
                onAddChild={(parentId) => setNewDeptParent(parentId)}
              />
            ))}

            {newDeptParent && (
              <p className="text-[11px] text-accent flex items-center justify-between">
                قسم فرعي من: {departments.find((d) => d.id === newDeptParent)?.name_ar}
                <button type="button" onClick={() => setNewDeptParent(null)} className="text-text-secondary hover:text-danger">
                  إلغاء
                </button>
              </p>
            )}
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
                <span className="truncate text-text-primary dark:text-white flex-1">{t.name}</span>
                <input
                  type="number"
                  min={1}
                  defaultValue={t.retention_years ?? ""}
                  onBlur={(e) => handleSetRetention(t.id, e.target.value)}
                  placeholder="دائم"
                  title="مدة الاحتفاظ بالسنوات (اتركه فارغًا = دائم)"
                  className="w-16 text-xs px-1.5 py-1 rounded-md border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white text-center"
                />
                <button onClick={() => handleDeleteType(t.id)} className="text-danger/70 hover:text-danger text-xs shrink-0">
                  حذف
                </button>
              </div>
            ))}
            <p className="text-[10px] text-text-secondary dark:text-white/30">
              الرقم بجانب كل نوع = مدة الاحتفاظ بالسنوات (فارغ = دائم، لا إتلاف تلقائي أبدًا)
            </p>
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
