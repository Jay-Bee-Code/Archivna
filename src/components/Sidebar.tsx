import Logo from "./Logo";
import NetworkStatus from "./NetworkStatus";
import {
  IconDashboard,
  IconArchive,
  IconSettings,
  IconRecycle,
  IconUsers,
  IconInbox,
} from "./icons/Icon";

export type View = "dashboard" | "archive" | "org" | "retention" | "users" | "inbox";

export default function Sidebar({
  view,
  onNavigate,
  isAdmin,
  inboxCount,
}: {
  view: View;
  onNavigate: (v: View) => void;
  isAdmin: boolean;
  inboxCount: number;
}) {
  const items: { id: View; label: string; icon: React.ReactNode; adminOnly?: boolean }[] = [
    { id: "dashboard", label: "لوحة المعلومات", icon: <IconDashboard size={17} /> },
    { id: "archive", label: "الأرشيف", icon: <IconArchive size={17} /> },
    { id: "inbox", label: "إحالاتي", icon: <IconInbox size={17} /> },
    { id: "org", label: "الهيكل التنظيمي", icon: <IconSettings size={17} />, adminOnly: true },
    { id: "retention", label: "الاحتفاظ والإتلاف", icon: <IconRecycle size={17} />, adminOnly: true },
    { id: "users", label: "المستخدمون", icon: <IconUsers size={17} />, adminOnly: true },
  ];

  return (
    <aside className="w-60 shrink-0 h-screen sticky top-0 border-e border-border-light dark:border-white/10 bg-white dark:bg-bg-dark/60 backdrop-blur flex flex-col">
      <div className="flex items-center gap-2.5 px-5 py-4">
        <Logo size={26} />
        <h1 className="text-sm font-bold text-primary dark:text-white leading-tight">
          نظام الأرشفة
          <br />
          الحكومي
        </h1>
      </div>

      <nav className="flex-1 px-3 flex flex-col gap-0.5 mt-2">
        {items.map((item) => {
          if (item.adminOnly && !isAdmin) return null;
          const active = view === item.id;
          return (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id)}
              className={`relative flex items-center gap-2.5 px-3 py-2.5 rounded-lg text-sm transition-colors text-start ${
                active
                  ? "bg-primary dark:bg-accent/25 text-white font-medium"
                  : "text-text-primary dark:text-white/70 hover:bg-black/5 dark:hover:bg-white/5"
              }`}
            >
              {item.icon}
              {item.label}
              {item.id === "inbox" && inboxCount > 0 && (
                <span
                  className={`ms-auto text-[10px] min-w-[18px] h-[18px] px-1 rounded-full flex items-center justify-center ${
                    active ? "bg-white/20 text-white" : "bg-accent text-white"
                  }`}
                >
                  {inboxCount > 99 ? "99+" : inboxCount}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      <div className="px-4 py-3 border-t border-border-light dark:border-white/10">
        <NetworkStatus />
      </div>
    </aside>
  );
}
