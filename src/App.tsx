import { useEffect, useState, useCallback } from "react";
import { api, Document, Category, UserPublic, errorMessage } from "./lib/api";
import DocumentCard from "./components/DocumentCard";
import CategorySidebar from "./components/CategorySidebar";
import VaultUnlock from "./components/VaultUnlock";
import AdminSetup from "./components/AdminSetup";
import SyncSetup from "./components/SyncSetup";
import LoginScreen from "./components/LoginScreen";
import NetworkStatus from "./components/NetworkStatus";
import EmptyState from "./components/EmptyState";
import Logo from "./components/Logo";
import UploadModal from "./components/UploadModal";
import OrgSettingsPanel from "./components/OrgSettingsPanel";

type Stage = "checking" | "unlock" | "setup-admin" | "sync-setup" | "login" | "ready";

const ROLE_LABELS: Record<string, string> = {
  admin: "مدير",
  archivist: "موظف أرشفة",
  reviewer: "مراجع",
  viewer: "قراءة فقط",
};

function Archive({ user, onLogout }: { user: UserPublic; onLogout: () => void }) {
  const [documents, setDocuments] = useState<Document[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canWrite = user.role === "admin" || user.role === "archivist";
  const [ocrAvailable, setOcrAvailable] = useState<boolean | null>(null);

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

  const [pendingFile, setPendingFile] = useState<File | null>(null);
  const [showOrgSettings, setShowOrgSettings] = useState(false);

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
    <div className="min-h-screen flex flex-col">
      <header className="border-b border-border-light dark:border-white/10 bg-white dark:bg-bg-dark/60 backdrop-blur px-6 py-3.5 flex items-center justify-between sticky top-0 z-20">
        <div className="flex items-center gap-2.5">
          <Logo size={26} />
          <h1 className="text-base font-bold text-primary dark:text-white">نظام الأرشفة الحكومي</h1>
        </div>

        <div className="flex items-center gap-4">
          <NetworkStatus />
          <div className="h-4 w-px bg-border-light dark:bg-white/10" />
          <div className="flex items-center gap-2 text-sm">
            <span className="text-text-primary dark:text-white/90">
              {user.full_name || user.username}
            </span>
            <span className="text-[11px] px-2 py-0.5 rounded-full bg-primary/10 dark:bg-accent/20 text-primary dark:text-accent">
              {ROLE_LABELS[user.role] || user.role}
            </span>
          </div>
          {user.role === "admin" && (
            <button
              onClick={() => setShowOrgSettings(true)}
              className="text-xs text-text-secondary dark:text-white/50 hover:text-primary dark:hover:text-white transition-colors"
              title="الهيكل التنظيمي وأنواع الوثائق"
            >
              ⚙️ الإعدادات
            </button>
          )}
          <button
            onClick={onLogout}
            className="text-xs text-text-secondary dark:text-white/50 hover:text-danger dark:hover:text-danger transition-colors"
          >
            خروج
          </button>
        </div>
      </header>

      <div className="flex-1 flex">
        <aside className="w-56 border-e border-border-light dark:border-white/10 p-4 hidden md:flex md:flex-col md:gap-4">
          {canWrite && (
            <label className="block cursor-pointer group">
              <span className="flex items-center justify-center gap-2 w-full text-center py-2.5 px-4 rounded-lg bg-primary text-white text-sm font-medium shadow-sm hover:bg-accent hover:shadow transition-all">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" className="opacity-90">
                  <path d="M8 3v7M4.5 6.5L8 3l3.5 3.5" stroke="white" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
                  <path d="M3 12.5h10" stroke="white" strokeWidth="1.5" strokeLinecap="round" />
                </svg>
                رفع وثيقة جديدة
              </span>
              <input type="file" onChange={handlePickFile} className="sr-only" />
            </label>
          )}
          {canWrite && ocrAvailable === false && (
            <p className="text-[11px] text-text-secondary dark:text-white/40 leading-relaxed">
              OCR غير مفعَّل على هذا الجهاز — النصوص داخل الصور لن تُفهرَس للبحث. راجع SETUP.md لتثبيت Tesseract.
            </p>
          )}

          <CategorySidebar
            categories={categories}
            activeCategory={activeCategory}
            onSelect={setActiveCategory}
            onAddCategory={canWrite ? handleAddCategory : async () => {}}
          />
        </aside>

        <main className="flex-1 p-6">
          <div className="flex items-center gap-3 mb-6">
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="بحث في الأرشيف… (العنوان والمحتوى)"
              className="flex-1 max-w-xl px-4 py-2.5 rounded-lg border border-border-light dark:border-white/10 focus:outline-none focus:ring-2 focus:ring-accent bg-white dark:bg-white/5 dark:text-white placeholder:text-text-secondary/70 dark:placeholder:text-white/30"
            />
            {activeCategoryName && (
              <span className="text-xs px-3 py-1.5 rounded-full bg-accent/10 text-accent whitespace-nowrap">
                الفئة: {activeCategoryName}
              </span>
            )}
          </div>

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
                    ? "ابدأ برفع وثيقة من الشريط الجانبي."
                    : undefined
              }
            />
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
              {documents.map((doc) => (
                <DocumentCard key={doc.id} doc={doc} canWrite={canWrite} onDelete={handleDelete} />
              ))}
            </div>
          )}
        </main>
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
      {showOrgSettings && <OrgSettingsPanel onClose={() => setShowOrgSettings(false)} />}
    </div>
  );
}

export default function App() {
  const [stage, setStage] = useState<Stage>("checking");
  const [user, setUser] = useState<UserPublic | null>(null);

  // عند إعادة تحميل الواجهة (dev mode)، تحقق إن كانت هناك جلسة مفتوحة أصلًا في الـ backend
  useEffect(() => {
    api
      .currentUser()
      .then((u) => {
        if (u) {
          setUser(u);
          setStage("ready");
        } else {
          setStage("unlock");
        }
      })
      .catch(() => setStage("unlock"));
  }, []);

  if (stage === "checking") {
    return <div className="min-h-screen flex items-center justify-center text-text-secondary">جارٍ التحقق…</div>;
  }

  if (stage === "unlock") {
    return (
      <VaultUnlock
        onUnlocked={(hasAdmin) => setStage(hasAdmin ? "login" : "setup-admin")}
      />
    );
  }

  if (stage === "setup-admin") {
    return <AdminSetup onCreated={() => setStage("sync-setup")} />;
  }

  if (stage === "sync-setup") {
    return <SyncSetup onDone={() => setStage("login")} />;
  }

  if (stage === "login") {
    return (
      <LoginScreen
        onLoggedIn={(u) => {
          setUser(u);
          setStage("ready");
        }}
      />
    );
  }

  return (
    <Archive
      user={user!}
      onLogout={async () => {
        await api.logout();
        setUser(null);
        setStage("login");
      }}
    />
  );
}
