import { useEffect, useState } from "react";
import { api, Document, UserPublic, errorMessage } from "../lib/api";
import {
  IconArchive,
  IconInbox,
  IconRecycle,
  IconScale,
  IconNote,
  IconDashboard,
} from "../components/icons/Icon";

function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString("ar-DZ", { year: "numeric", month: "short", day: "numeric" });
}

function StatCard({
  icon,
  label,
  value,
  hint,
  tone = "neutral",
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  value: number | string;
  hint?: string;
  tone?: "neutral" | "warning" | "accent";
  onClick?: () => void;
}) {
  const toneClass =
    tone === "warning"
      ? "bg-warning/10 text-warning"
      : tone === "accent"
        ? "bg-accent/10 text-accent"
        : "bg-primary/10 dark:bg-white/5 text-primary dark:text-white/70";
  const Tag = onClick ? "button" : "div";
  return (
    <Tag
      onClick={onClick}
      className={`text-start bg-white dark:bg-white/[0.03] border border-border-light dark:border-white/10 rounded-2xl p-5 flex flex-col gap-3 ${
        onClick ? "hover:border-accent/40 hover:shadow-md transition-all cursor-pointer" : ""
      }`}
    >
      <span className={`w-9 h-9 rounded-xl flex items-center justify-center ${toneClass}`}>{icon}</span>
      <div>
        <p className="text-2xl font-bold text-text-primary dark:text-white tabular-nums">{value}</p>
        <p className="text-sm text-text-secondary dark:text-white/50">{label}</p>
        {hint && <p className="text-[11px] text-text-secondary/70 dark:text-white/30 mt-0.5">{hint}</p>}
      </div>
    </Tag>
  );
}

export default function DashboardPage({
  user,
  inboxCount,
  onNavigate,
}: {
  user: UserPublic;
  inboxCount: number;
  onNavigate: (v: "archive" | "inbox" | "retention") => void;
}) {
  const [docs, setDocs] = useState<Document[] | null>(null);
  const [disposalCount, setDisposalCount] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const isAdmin = user.role === "admin";

  useEffect(() => {
    api.listDocuments().then(setDocs).catch((e) => setError(errorMessage(e)));
    if (isAdmin) {
      api.listDisposalCandidates().then((d) => setDisposalCount(d.length)).catch(() => setDisposalCount(null));
    }
  }, [isAdmin]);

  const legalHoldCount = docs?.filter((d) => d.legal_hold).length ?? 0;
  const ocrCount = docs?.filter((d) => d.has_ocr).length ?? 0;
  const recent = docs?.slice(0, 5) ?? [];

  const greeting = user.full_name || user.username;

  return (
    <div className="max-w-5xl mx-auto p-6 md:p-8 flex flex-col gap-6">
      <div>
        <h2 className="flex items-center gap-2.5 text-xl font-bold text-primary dark:text-white">
          <IconDashboard size={22} /> مرحبًا، {greeting}
        </h2>
        <p className="text-sm text-text-secondary dark:text-white/50 mt-1">نظرة سريعة على حالة الأرشيف.</p>
      </div>

      {error && <p className="text-sm text-danger bg-danger/10 rounded-md p-3">{error}</p>}

      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={<IconArchive size={18} />}
          label="وثيقة في أرشيفك"
          value={docs?.length ?? "…"}
          onClick={() => onNavigate("archive")}
        />
        <StatCard
          icon={<IconInbox size={18} />}
          label="إحالة إليك"
          value={inboxCount}
          tone="accent"
          onClick={() => onNavigate("inbox")}
        />
        {isAdmin && (
          <StatCard
            icon={<IconRecycle size={18} />}
            label="مؤهَّلة للإتلاف"
            value={disposalCount ?? "…"}
            hint="انتهت مدة احتفاظها"
            tone={disposalCount && disposalCount > 0 ? "warning" : "neutral"}
            onClick={() => onNavigate("retention")}
          />
        )}
        <StatCard
          icon={<IconScale size={18} />}
          label="مجمَّدة قانونيًا"
          value={legalHoldCount}
        />
        {!isAdmin && (
          <StatCard icon={<IconNote size={18} />} label="بنص مستخرَج (OCR)" value={ocrCount} />
        )}
      </div>

      <section className="bg-white dark:bg-white/[0.03] border border-border-light dark:border-white/10 rounded-2xl p-5">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-xs font-semibold text-text-secondary dark:text-white/40 uppercase tracking-wide">
            آخر الوثائق
          </h3>
          <button onClick={() => onNavigate("archive")} className="text-xs text-accent hover:underline">
            عرض الكل
          </button>
        </div>
        {recent.length === 0 ? (
          <p className="text-sm text-text-secondary dark:text-white/40">لا وثائق بعد.</p>
        ) : (
          <div className="flex flex-col gap-1.5">
            {recent.map((d) => (
              <div
                key={d.id}
                className="flex items-center justify-between gap-3 px-3 py-2 rounded-lg bg-black/[0.02] dark:bg-white/[0.03]"
              >
                <span className="text-sm text-text-primary dark:text-white truncate">{d.title}</span>
                <span className="text-[11px] text-text-secondary dark:text-white/40 shrink-0">
                  {d.registry_number ? `${d.registry_number} • ` : ""}
                  {formatDate(d.created_at)}
                </span>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
