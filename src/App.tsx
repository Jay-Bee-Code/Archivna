import { useEffect, useState, useCallback } from "react";
import { api, UserPublic } from "./lib/api";
import VaultUnlock from "./components/VaultUnlock";
import AdminSetup from "./components/AdminSetup";
import SyncSetup from "./components/SyncSetup";
import LoginScreen from "./components/LoginScreen";
import Sidebar, { View } from "./components/Sidebar";
import { IconLogout } from "./components/icons/Icon";
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

  return (
    <div className="h-screen flex overflow-hidden">
      <Sidebar view={view} onNavigate={setView} isAdmin={isAdmin} inboxCount={inboxCount} />

      <div className="flex-1 flex flex-col min-w-0">
        <header className="shrink-0 border-b border-border-light dark:border-white/10 bg-white dark:bg-bg-dark/60 backdrop-blur px-6 py-3 flex items-center justify-end gap-4">
          <div className="flex items-center gap-2 text-sm">
            <span className="text-text-primary dark:text-white/90">{user.full_name || user.username}</span>
            <span className="text-[11px] px-2 py-0.5 rounded-full bg-primary/10 dark:bg-accent/20 text-primary dark:text-accent">
              {ROLE_LABELS[user.role] || user.role}
            </span>
          </div>
          <button
            onClick={onLogout}
            className="flex items-center gap-1.5 text-xs text-text-secondary dark:text-white/50 hover:text-danger px-2.5 py-1.5 rounded-lg hover:bg-danger/5 transition-colors"
          >
            <IconLogout size={14} /> خروج
          </button>
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
