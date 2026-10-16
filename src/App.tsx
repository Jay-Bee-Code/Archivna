import { useEffect, useState, useCallback } from "react";
import { api, UserPublic } from "./lib/api";
import VaultUnlock from "./components/VaultUnlock";
import AdminSetup from "./components/AdminSetup";
import SyncSetup from "./components/SyncSetup";
import LoginScreen from "./components/LoginScreen";
import Sidebar, { View } from "./components/Sidebar";
import { IconLogout, IconSearch } from "./components/icons/Icon";
import DashboardPage from "./pages/DashboardPage";
import ArchivePage from "./pages/ArchivePage";
import InboxPage from "./pages/InboxPage";
import UsersPage from "./pages/UsersPage";
import OrgSettingsPanel from "./components/OrgSettingsPanel";
import RetentionPanel from "./components/RetentionPanel";

type Stage = "checking" | "unlock" | "setup-admin" | "sync-setup" | "login" | "ready";

const ROLE_LABELS: Record<string, string> = {
  admin: "مدير",
  archivist: "موظف أرشفة",
  reviewer: "مراجع",
  viewer: "قراءة فقط",
};

/** الغلاف الرئيسي: شريط جانبي دائم + شريط علوي + صفحة نشطة (بدل صفحة واحدة ونوافذ لكل شيء) */
function AppShell({ user, onLogout }: { user: UserPublic; onLogout: () => void }) {
  const [view, setView] = useState<View>("dashboard");
  const [archiveQuery, setArchiveQuery] = useState("");
  const [inboxCount, setInboxCount] = useState(0);
  const isAdmin = user.role === "admin";

  const refreshInbox = useCallback(() => {
    api.listMyInbox().then((items) => setInboxCount(items.length)).catch(() => {});
  }, []);

  // عدّاد الإحالات في الشريط الجانبي: عند الدخول، وعند كل تنقّل، ودوريًا (المزامنة قد تُحضر إحالات جديدة)
  useEffect(() => {
    refreshInbox();
  }, [view, refreshInbox]);
  useEffect(() => {
    const t = setInterval(refreshInbox, 20000);
    return () => clearInterval(t);
  }, [refreshInbox]);

  function openInArchive(title: string) {
    setArchiveQuery(title);
    setView("archive");
  }

  const content = (() => {
    switch (view) {
      case "dashboard":
        return <DashboardPage user={user} inboxCount={inboxCount} onNavigate={setView} />;
      case "archive":
        return <ArchivePage user={user} query={archiveQuery} onQueryChange={setArchiveQuery} />;
      case "inbox":
        return <InboxPage onOpenInArchive={openInArchive} />;
      case "org":
        return isAdmin ? <OrgSettingsPanel /> : null;
      case "retention":
        return isAdmin ? <RetentionPanel /> : null;
      case "users":
        return isAdmin ? <UsersPage /> : null;
    }
  })();

  const roleLabel = ROLE_LABELS[user.role] || user.role;
  const displayName = user.full_name || user.username;
  const initials = displayName.trim().split(/\s+/).slice(0, 2).map((w) => w[0]).join("");

  return (
    <div className="h-screen flex overflow-hidden bg-page">
      <Sidebar view={view} onNavigate={setView} isAdmin={isAdmin} inboxCount={inboxCount} />

      <div className="flex-1 flex flex-col min-w-0">
        {/* الهيدر: البحث هو المركز، ويعيش هنا في كل الصفحات */}
        <header className="shrink-0 h-14 border-b border-line bg-raised flex items-center gap-4 px-6">
          <div className="relative flex-1 max-w-xl">
            <IconSearch size={15} className="absolute top-1/2 -translate-y-1/2 start-3 text-faint pointer-events-none" />
            <input
              type="search"
              value={archiveQuery}
              onChange={(e) => setArchiveQuery(e.target.value)}
              onFocus={() => view !== "archive" && setView("archive")}
              placeholder="ابحث في الوثائق بالعنوان أو المحتوى…"
              aria-label="بحث في الأرشيف"
              className="w-full h-9 ps-9 pe-3 rounded-md bg-sunken border border-transparent text-sm text-ink placeholder:text-faint focus:outline-none focus:bg-raised focus:border-accent focus:ring-2 focus:ring-accent-soft"
            />
          </div>

          <div className="ms-auto flex items-center gap-3">
            <div className="flex items-center gap-2.5">
              <span
                aria-hidden
                className="w-8 h-8 rounded-full bg-accent-soft text-accent text-xs font-semibold flex items-center justify-center"
              >
                {initials}
              </span>
              <div className="leading-tight">
                <p className="text-sm font-medium text-ink">{displayName}</p>
                <p className="text-xs text-muted">{roleLabel}</p>
              </div>
            </div>
            <span className="w-px h-6 bg-line" aria-hidden />
            <button
              onClick={onLogout}
              className="flex items-center gap-1.5 h-8 px-2.5 rounded-md text-sm text-muted hover:text-danger hover:bg-danger-soft"
            >
              <IconLogout size={14} /> خروج
            </button>
          </div>
        </header>

        <main className="flex-1 overflow-y-auto">{content}</main>
      </div>
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
    <AppShell
      user={user!}
      onLogout={async () => {
        await api.logout();
        setUser(null);
        setStage("login");
      }}
    />
  );
}
