import { useEffect, useState } from "react";
import { api, Category, Document, UserPublic, errorMessage } from "../lib/api";
import Bidi from "../components/Bidi";
import { IconInbox, IconRecycle, IconUpload, IconArchive } from "../components/icons/Icon";

/** الاسم المعروض بلا امتداد الملف التقني. الاسم المخزَّن لا يُعدَّل. */
function displayTitle(title: string): string {
  const dot = title.lastIndexOf(".");
  return dot > 0 && dot >= title.length - 5 ? title.slice(0, dot) : title;
}

/**
 * الرئيسية: تجيب سؤالًا واحدًا — "ماذا أفعل الآن؟"
 * - إجراء واحد بارز (الرفع)، لأن الرفع أكثر ما يقوم به موظف الأرشيف.
 * - "يحتاج انتباهك": ما ينتظر قرارًا، ولا يظهر إن لم يكن هناك شيء.
 * - المجلدات: الفئات كبطاقات يمكن الانتقال إليها، بدل أصفار لا تعني شيئًا.
 * - آخر الوثائق: قائمة قصيرة بلغة المستخدم، والتفاصيل التقنية في الدرج.
 */
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
  const [folders, setFolders] = useState<Category[]>([]);
  const [disposalCount, setDisposalCount] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const isAdmin = user.role === "admin";
  const canWrite = isAdmin || user.role === "archivist";

  useEffect(() => {
    api.listDocuments().then(setDocs).catch((e) => setError(errorMessage(e)));
    api.listCategories().then(setFolders).catch(() => {});
    if (isAdmin) {
      api.listDisposalCandidates().then((d) => setDisposalCount(d.length)).catch(() => setDisposalCount(null));
    }
  }, [isAdmin]);

  const firstName = (user.full_name || user.username).split(/\s+/)[0];
  const hour = new Date().getHours();
  const greeting = hour < 12 ? "صباح الخير" : hour < 18 ? "مساء الخير" : "مساء النور";

  // "يحتاج انتباهك": عناصر تستحق قرارًا الآن. تُعرض فقط إن وُجدت.
  const attention: { key: string; icon: React.ReactNode; title: string; detail: string; tone: "warning" | "accent"; onClick: () => void }[] = [];
  if (isAdmin && disposalCount && disposalCount > 0) {
    attention.push({
      key: "dispose",
      icon: <IconRecycle size={16} />,
      title: `${disposalCount} ${disposalCount === 1 ? "وثيقة" : "وثائق"} انتهت مدة احتفاظها`,
      detail: "تحتاج قرارك قبل الإتلاف",
      tone: "warning",
      onClick: () => onNavigate("retention"),
    });
  }
  if (inboxCount > 0) {
    attention.push({
      key: "inbox",
      icon: <IconInbox size={16} />,
      title: `${inboxCount} ${inboxCount === 1 ? "إحالة تنتظرك" : "إحالات تنتظرك"}`,
      detail: "وثائق أحالها إليك زملاؤك",
      tone: "accent",
      onClick: () => onNavigate("inbox"),
    });
  }

  const recent = docs?.slice(0, 6) ?? [];

  return (
    <div className="mx-auto max-w-4xl px-6 py-8 flex flex-col gap-8">
      {/* الترحيب + الإجراء الأساسي */}
      <section className="flex items-end justify-between gap-6">
        <div>
          <p className="text-sm text-muted">{greeting}، {firstName}</p>
          <h1 className="text-xl font-semibold text-ink mt-1">ماذا تريد أن تفعل اليوم؟</h1>
        </div>
        {canWrite && (
          <button
            onClick={() => onNavigate("archive")}
            className="shrink-0 flex items-center gap-2 h-10 px-4 rounded-md bg-accent text-on-accent text-sm font-medium shadow-sm hover:bg-accent-strong"
          >
            <IconUpload size={15} /> رفع وثيقة
          </button>
        )}
      </section>

      {error && <p className="text-sm text-danger">{error}</p>}

      {/* يحتاج انتباهك: يظهر فقط عند وجود شيء يستحق قرارًا */}
      {attention.length > 0 && (
        <section aria-labelledby="attention-title" className="flex flex-col gap-3">
          <h2 id="attention-title" className="text-sm font-medium text-ink">يحتاج انتباهك</h2>
          <div className="flex flex-col gap-2">
            {attention.map((a) => (
              <button
                key={a.key}
                onClick={a.onClick}
                className="group flex items-center gap-4 p-4 rounded-lg border border-line bg-raised text-start hover:border-line-strong hover:shadow-sm"
              >
                <span
                  className={`shrink-0 w-9 h-9 rounded-md flex items-center justify-center ${
                    a.tone === "warning" ? "bg-warning-soft text-warning" : "bg-accent-soft text-accent"
                  }`}
                >
                  {a.icon}
                </span>
                <span className="flex-1 min-w-0">
                  <span className="block text-sm font-medium text-ink">{a.title}</span>
                  <span className="block text-xs text-muted mt-0.5">{a.detail}</span>
                </span>
                <span className="text-muted opacity-0 group-hover:opacity-100 group-hover:text-accent transition-opacity">←</span>
              </button>
            ))}
          </div>
        </section>
      )}

      {/* المجلدات: الفئات كأشياء ملموسة يمكن فتحها */}
      <section aria-labelledby="folders-title" className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <h2 id="folders-title" className="text-sm font-medium text-ink">المجلدات</h2>
          <button onClick={() => onNavigate("archive")} className="text-xs text-accent hover:underline">
            عرض الكل
          </button>
        </div>
        {folders.length === 0 ? (
          <div className="rounded-lg border border-dashed border-line-strong p-6 text-center">
            <IconArchive size={20} />
            <p className="text-sm text-body mt-2">لا توجد مجلدات بعد</p>
            <p className="text-xs text-muted mt-1">تُنشأ المجلدات من صفحة الأرشيف لتنظيم الوثائق حسب موضوعها.</p>
          </div>
        ) : (
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3 [&>*]:w-full">
            {folders.slice(0, 8).map((f) => (
              <button
                key={f.id}
                onClick={() => onNavigate("archive")}
                className="flex flex-col items-start gap-3 p-4 rounded-lg border border-line bg-raised text-start hover:border-line-strong hover:shadow-sm"
              >
                <span className="w-9 h-9 rounded-md bg-sunken text-muted flex items-center justify-center">
                  <IconArchive size={16} />
                </span>
                <span className="w-full">
                  <span className="block text-sm font-medium text-ink truncate">
                    <Bidi>{f.name}</Bidi>
                  </span>
                  <span className="block text-xs text-muted tabular-nums mt-0.5">
                    {f.document_count} {f.document_count === 1 ? "وثيقة" : "وثائق"}
                  </span>
                </span>
              </button>
            ))}
          </div>
        )}
      </section>

      {/* آخر الوثائق: قائمة بلغة المستخدم، بلا تفاصيل تقنية */}
      <section aria-labelledby="recent-title" className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <h2 id="recent-title" className="text-sm font-medium text-ink">أُضيفت مؤخرًا</h2>
          <button onClick={() => onNavigate("archive")} className="text-xs text-accent hover:underline">
            كل الوثائق
          </button>
        </div>
        {docs === null && !error ? (
          <p className="text-sm text-muted">جارٍ التحميل…</p>
        ) : recent.length === 0 ? (
          <p className="text-sm text-muted py-4">لم تُرفع أي وثيقة بعد.</p>
        ) : (
          <ul className="divide-y divide-line rounded-lg border border-line bg-raised overflow-hidden">
            {recent.map((d) => (
              <li key={d.id}>
                <button
                  onClick={() => onNavigate("archive")}
                  className="w-full flex items-center gap-4 px-4 h-12 text-start hover:bg-hover"
                >
                  <span className="flex-1 min-w-0 truncate text-sm text-ink">
                    <Bidi>{displayTitle(d.title)}</Bidi>
                  </span>
                  <span className="text-xs text-muted tabular-nums shrink-0">
                    {new Date(d.created_at * 1000).toLocaleDateString("ar-DZ", { day: "numeric", month: "short" })}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
