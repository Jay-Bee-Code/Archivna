import { useEffect, useState, useCallback } from "react";
import { api, Document, Category, UserPublic, errorMessage } from "../lib/api";
import DocumentCard from "../components/DocumentCard";
import CategorySidebar from "../components/CategorySidebar";
import UploadModal from "../components/UploadModal";
import EmptyState from "../components/EmptyState";
import { IconSearch, IconUpload } from "../components/icons/Icon";

export default function ArchivePage({
  user,
  query,
  onQueryChange,
}: {
  user: UserPublic;
  query: string;
  onQueryChange: (q: string) => void;
}) {
  const [documents, setDocuments] = useState<Document[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pendingFile, setPendingFile] = useState<File | null>(null);
  const [ocrAvailable, setOcrAvailable] = useState<boolean | null>(null);

  const canWrite = user.role === "admin" || user.role === "archivist";

  useEffect(() => {
    api.ocrStatus().then(setOcrAvailable).catch(() => setOcrAvailable(false));
  }, []);

  const loadCategories = useCallback(async () => {
    try {
      setCategories(await api.listCategories());
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  const loadDocuments = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      let docs: Document[];
      if (query.trim()) {
        docs = await api.searchDocuments(query.trim());
        if (activeCategory) docs = docs.filter((d) => d.category_id === activeCategory);
      } else if (activeCategory) {
        docs = await api.listDocumentsByCategory(activeCategory);
      } else {
        docs = await api.listDocuments();
      }
      setDocuments(docs);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setLoading(false);
    }
  }, [query, activeCategory]);

  useEffect(() => {
    loadCategories();
  }, [loadCategories]);

  useEffect(() => {
    const timeout = setTimeout(loadDocuments, 200);
    return () => clearTimeout(timeout);
  }, [loadDocuments]);

  function handlePickFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (file) setPendingFile(file);
    e.target.value = "";
  }

  async function handleUploaded() {
    setPendingFile(null);
    await Promise.all([loadDocuments(), loadCategories()]);
  }

  async function handleDelete(id: string) {
    try {
      await api.deleteDocument(id);
      await Promise.all([loadDocuments(), loadCategories()]);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleAddCategory(name: string) {
    await api.addCategory({ name, parent_id: null });
    await loadCategories();
  }

  const activeCategoryName = categories.find((c) => c.id === activeCategory)?.name;

  return (
    <div className="flex h-full">
      <aside className="w-52 shrink-0 border-e border-border-light dark:border-white/10 p-4 hidden md:block overflow-y-auto">
        <CategorySidebar
          categories={categories}
          activeCategory={activeCategory}
          onSelect={setActiveCategory}
          onAddCategory={canWrite ? handleAddCategory : async () => {}}
        />
      </aside>

      <div className="flex-1 p-6 overflow-y-auto">
        <div className="flex items-center gap-3 mb-6">
          <div className="relative flex-1 max-w-xl">
            <IconSearch
              size={15}
              className="absolute top-1/2 -translate-y-1/2 start-3.5 text-text-secondary/50 dark:text-white/30 pointer-events-none"
            />
            <input
              type="text"
              value={query}
              onChange={(e) => onQueryChange(e.target.value)}
              placeholder="بحث في الأرشيف… (العنوان والمحتوى)"
              className="w-full ps-9 pe-4 py-2.5 rounded-lg border border-border-light dark:border-white/10 focus:outline-none focus:ring-2 focus:ring-accent bg-white dark:bg-white/5 dark:text-white placeholder:text-text-secondary/70 dark:placeholder:text-white/30"
            />
          </div>

          {activeCategoryName && (
            <span className="text-xs px-3 py-1.5 rounded-full bg-accent/10 text-accent whitespace-nowrap">
              الفئة: {activeCategoryName}
            </span>
          )}

          {canWrite && (
            <label className="shrink-0 cursor-pointer ms-auto">
              <span className="flex items-center gap-2 py-2.5 px-4 rounded-lg bg-primary text-white text-sm font-medium shadow-sm hover:bg-accent hover:shadow transition-all">
                <IconUpload size={15} />
                رفع وثيقة
              </span>
              <input type="file" onChange={handlePickFile} className="sr-only" />
            </label>
          )}
        </div>

        {canWrite && ocrAvailable === false && (
          <p className="text-[11px] text-text-secondary dark:text-white/40 mb-4">
            OCR غير مفعَّل على هذا الجهاز — النصوص داخل الصور لن تُفهرَس للبحث. راجع SETUP.md لتثبيت Tesseract.
          </p>
        )}

        {error && <div className="mb-4 text-sm text-danger bg-danger/10 rounded-md p-3">{error}</div>}

        {loading && documents.length === 0 ? (
          <div className="flex items-center gap-2 text-text-secondary dark:text-white/40 text-sm py-10 justify-center">
            <span className="w-3.5 h-3.5 rounded-full border-2 border-current border-t-transparent animate-spin" />
            جارٍ التحميل…
          </div>
        ) : documents.length === 0 ? (
          <EmptyState
            title={query.trim() ? "لا نتائج مطابقة" : "لا توجد وثائق بعد"}
            hint={
              query.trim()
                ? "جرّب كلمات بحث أخرى، أو تحقق من الفئة المحددة."
                : canWrite
                  ? "ابدأ برفع وثيقة من الزر أعلاه."
                  : undefined
            }
          />
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
            {documents.map((doc) => (
              <DocumentCard key={doc.id} doc={doc} canWrite={canWrite} onDelete={handleDelete} />
            ))}
          </div>
        )}
      </div>

      {pendingFile && (
        <UploadModal
          file={pendingFile}
          categories={categories}
          defaultCategoryId={activeCategory}
          defaultDepartmentId={user.department_id}
          onClose={() => setPendingFile(null)}
          onUploaded={handleUploaded}
        />
      )}
    </div>
  );
}
