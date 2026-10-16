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

type Item = { id: View; label: string; icon: React.ReactNode; adminOnly?: boolean };

/**
 * الشريط الجانبي: تنقّل فقط. لا أزرار إجراء هنا (الرفع في الصفحة نفسها).
 * المجموعات مفصولة بعنوان صغير، كما في المراجع الاحترافية.
 */
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
  const work: Item[] = [
    { id: "dashboard", label: "الرئيسية", icon: <IconDashboard size={16} /> },
    { id: "archive", label: "الأرشيف", icon: <IconArchive size={16} /> },
    { id: "inbox", label: "إحالاتي", icon: <IconInbox size={16} /> },
  ];
  const admin: Item[] = [
    { id: "org", label: "الهيكل التنظيمي", icon: <IconSettings size={16} />, adminOnly: true },
    { id: "retention", label: "الاحتفاظ والإتلاف", icon: <IconRecycle size={16} />, adminOnly: true },
    { id: "users", label: "المستخدمون", icon: <IconUsers size={16} />, adminOnly: true },
  ];

  const renderItem = (item: Item) => {
    const active = view === item.id;
    return (
      <button
        key={item.id}
        onClick={() => onNavigate(item.id)}
        aria-current={active ? "page" : undefined}
        className={`group relative w-full flex items-center gap-2.5 h-9 px-2.5 rounded-md text-sm text-start transition-colors ${
          active ? "bg-active text-ink font-medium" : "text-body hover:bg-hover hover:text-ink"
        }`}
      >
        {active && <span className="absolute inset-y-1.5 start-0 w-0.5 rounded-full bg-accent" aria-hidden />}
        <span className={active ? "text-accent" : "text-muted group-hover:text-body"}>{item.icon}</span>
        <span className="flex-1 truncate">{item.label}</span>
        {item.id === "inbox" && inboxCount > 0 && (
          <span className="text-xs tabular-nums min-w-[20px] h-5 px-1.5 rounded-full bg-accent text-on-accent flex items-center justify-center">
            {inboxCount > 99 ? "99+" : inboxCount}
          </span>
        )}
      </button>
    );
  };

  return (
    <aside className="w-56 shrink-0 h-screen sticky top-0 border-e border-line bg-raised flex flex-col">
      <div className="flex items-center gap-2.5 h-14 px-4 border-b border-line">
        <Logo size={22} />
        <span className="text-sm font-semibold text-ink leading-tight">نظام الأرشفة</span>
      </div>

      <nav className="flex-1 overflow-y-auto px-3 py-4 flex flex-col gap-6" aria-label="التنقّل الرئيسي">
        <div className="flex flex-col gap-0.5">
          <h2 className="px-2.5 pb-1.5 text-xs font-medium text-faint">العمل</h2>
          {work.map(renderItem)}
        </div>
        {isAdmin && (
          <div className="flex flex-col gap-0.5">
            <h2 className="px-2.5 pb-1.5 text-xs font-medium text-faint">الإدارة</h2>
            {admin.map(renderItem)}
          </div>
        )}
      </nav>

      <div className="px-4 py-3 border-t border-line">
        <NetworkStatus />
      </div>
    </aside>
  );
}
