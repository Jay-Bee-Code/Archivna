import { useEffect, useState } from "react";
import {
  api,
  Category,
  Department,
  DocumentType,
  CONFIDENTIALITY_LABELS,
  fileToBase64,
  errorMessage,
} from "../lib/api";
import { flattenDeptTree, deptIndent } from "../lib/deptTree";

export default function UploadModal({
  file,
  categories,
  defaultCategoryId,
  defaultDepartmentId,
  onClose,
  onUploaded,
}: {
  file: File;
  categories: Category[];
  defaultCategoryId: string | null;
  defaultDepartmentId: string | null;
  onClose: () => void;
  onUploaded: () => void;
}) {
  const [title, setTitle] = useState(file.name);
  const [categoryId, setCategoryId] = useState<string | null>(defaultCategoryId);
  const [departmentId, setDepartmentId] = useState<string | null>(defaultDepartmentId);
  const [documentTypeId, setDocumentTypeId] = useState<string | null>(null);
  const [confidentiality, setConfidentiality] = useState(1);

  const [departments, setDepartments] = useState<Department[]>([]);
  const [types, setTypes] = useState<DocumentType[]>([]);
  const [suggested, setSuggested] = useState<string[]>([]);
  const [addingType, setAddingType] = useState(false);
  const [newTypeName, setNewTypeName] = useState("");

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listDepartments().then(setDepartments).catch(() => {});
    api.listDocumentTypes().then(setTypes).catch(() => {});
    api.suggestedDocumentTypes().then(setSuggested).catch(() => {});
  }, []);

  async function handleAddType(name: string) {
    try {
      const t = await api.addDocumentType(name);
      setTypes((prev) => [...prev, t].sort((a, b) => a.name.localeCompare(b.name)));
      setDocumentTypeId(t.id);
      setAddingType(false);
      setNewTypeName("");
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const base64 = await fileToBase64(file);
      await api.addDocument({
        title: title.trim() || file.name,
        category_id: categoryId,
        file_base64: base64,
        mime_type: file.type || null,
        departmentId,
        documentTypeId,
        confidentialityLevel: confidentiality,
      });
      onUploaded();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  const selectClass =
    "w-full px-3 py-2 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white text-sm focus:outline-none focus:ring-2 focus:ring-accent";

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-40 px-4" onClick={onClose}>
      <form
        onSubmit={handleSubmit}
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-md bg-white dark:bg-[#132033] border border-border-light dark:border-white/10 rounded-2xl p-6 flex flex-col gap-3.5 max-h-[90vh] overflow-y-auto"
      >
        <h2 className="text-base font-bold text-primary dark:text-white">رفع وثيقة جديدة</h2>
        <p className="text-xs text-text-secondary dark:text-white/40 truncate">{file.name}</p>

        <label className="flex flex-col gap-1">
          <span className="text-xs text-text-secondary dark:text-white/50">العنوان</span>
          <input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className={selectClass}
            autoFocus
          />
        </label>

        <div className="grid grid-cols-2 gap-3">
          <label className="flex flex-col gap-1">
            <span className="text-xs text-text-secondary dark:text-white/50">الفئة</span>
            <select
              value={categoryId ?? ""}
              onChange={(e) => setCategoryId(e.target.value || null)}
              className={selectClass}
            >
              <option value="">بلا فئة</option>
              {categories.map((c) => (
                <option key={c.id} value={c.id}>{c.name}</option>
              ))}
            </select>
          </label>

          <label className="flex flex-col gap-1">
            <span className="text-xs text-text-secondary dark:text-white/50">القسم</span>
            <select
              value={departmentId ?? ""}
              onChange={(e) => setDepartmentId(e.target.value || null)}
              className={selectClass}
            >
              <option value="">بلا قسم (مركزي)</option>
              {flattenDeptTree(departments).map(({ dept, depth }) => (
                <option key={dept.id} value={dept.id}>{deptIndent(depth)}{dept.name_ar}</option>
              ))}
            </select>
          </label>
        </div>

        <label className="flex flex-col gap-1">
          <span className="text-xs text-text-secondary dark:text-white/50">نوع الوثيقة</span>
          {!addingType ? (
            <div className="flex gap-2">
              <select
                value={documentTypeId ?? ""}
                onChange={(e) => setDocumentTypeId(e.target.value || null)}
                className={selectClass}
              >
                <option value="">بلا نوع</option>
                {types.map((t) => (
                  <option key={t.id} value={t.id}>{t.name}</option>
                ))}
              </select>
              <button
                type="button"
                onClick={() => setAddingType(true)}
                className="shrink-0 text-xs px-3 rounded-lg border border-border-light dark:border-white/10 text-accent hover:bg-accent/5"
              >
                + جديد
              </button>
            </div>
          ) : (
            <div className="flex flex-col gap-2">
              <div className="flex gap-2">
                <input
                  value={newTypeName}
                  onChange={(e) => setNewTypeName(e.target.value)}
                  placeholder="اسم النوع"
                  className={selectClass}
                  autoFocus
                />
                <button
                  type="button"
                  disabled={!newTypeName.trim()}
                  onClick={() => handleAddType(newTypeName.trim())}
                  className="shrink-0 text-xs px-3 rounded-lg bg-accent text-white disabled:opacity-40"
                >
                  إضافة
                </button>
              </div>
              {suggested.length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                  {suggested
                    .filter((s) => !types.some((t) => t.name === s))
                    .map((s) => (
                      <button
                        key={s}
                        type="button"
                        onClick={() => handleAddType(s)}
                        className="text-[11px] px-2 py-1 rounded-full bg-black/5 dark:bg-white/5 text-text-secondary dark:text-white/60 hover:bg-accent/10 hover:text-accent"
                      >
                        {s}
                      </button>
                    ))}
                </div>
              )}
            </div>
          )}
        </label>

        <label className="flex flex-col gap-1">
          <span className="text-xs text-text-secondary dark:text-white/50">مستوى السرية</span>
          <select
            value={confidentiality}
            onChange={(e) => setConfidentiality(Number(e.target.value))}
            className={selectClass}
          >
            {Object.entries(CONFIDENTIALITY_LABELS).map(([level, label]) => (
              <option key={level} value={level}>{label}</option>
            ))}
          </select>
        </label>

        {error && <p className="text-sm text-danger">{error}</p>}

        <div className="flex gap-2 mt-1">
          <button
            type="button"
            onClick={onClose}
            className="flex-1 py-2.5 rounded-lg border border-border-light dark:border-white/10 text-text-secondary dark:text-white/60 text-sm hover:bg-black/5 dark:hover:bg-white/5"
          >
            إلغاء
          </button>
          <button
            type="submit"
            disabled={loading}
            className="flex-1 py-2.5 rounded-lg bg-primary text-white text-sm font-medium hover:bg-accent shadow-sm disabled:opacity-50"
          >
            {loading ? "جارٍ الرفع…" : "رفع"}
          </button>
        </div>
      </form>
    </div>
  );
}
