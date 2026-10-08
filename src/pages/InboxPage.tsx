import { useEffect, useState } from "react";
import { api, InboxEntry, errorMessage } from "../lib/api";
import EmptyState from "../components/EmptyState";
import { IconInbox, IconSearch } from "../components/icons/Icon";

function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString("ar-DZ", { year: "numeric", month: "long", day: "numeric" });
}

export default function InboxPage({ onOpenInArchive }: { onOpenInArchive: (title: string) => void }) {
  const [items, setItems] = useState<InboxEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listMyInbox().then(setItems).catch((e) => setError(errorMessage(e)));
  }, []);

  return (
    <div className="max-w-3xl mx-auto p-6 md:p-8 flex flex-col gap-5">
      <h2 className="flex items-center gap-2.5 text-xl font-bold text-primary dark:text-white">
        <IconInbox size={22} /> إحالاتي
      </h2>
      <p className="text-sm text-text-secondary dark:text-white/50 -mt-3">
        الوثائق التي أُحيلت إليك من زملائك داخل المؤسسة، الأحدث أولًا.
      </p>

      {error && <p className="text-sm text-danger bg-danger/10 rounded-md p-3">{error}</p>}

      {items === null && !error ? (
        <p className="text-sm text-text-secondary dark:text-white/40">جارٍ التحميل…</p>
      ) : items && items.length === 0 ? (
        <EmptyState title="لا إحالات إليك بعد" hint="عندما يُحيل لك زميل وثيقة، ستظهر هنا." />
      ) : (
        <div className="flex flex-col gap-2.5">
          {items?.map((it) => (
            <div
              key={it.id}
              className="bg-white dark:bg-white/[0.03] border border-border-light dark:border-white/10 rounded-xl p-4 flex items-start justify-between gap-3"
            >
              <div className="min-w-0">
                <p className="font-medium text-text-primary dark:text-white truncate">{it.document_title}</p>
                <p className="text-xs text-text-secondary dark:text-white/40 mt-0.5">
                  من {it.routed_by_username || "—"} • {formatDate(it.routed_at)}
                </p>
                {it.note && (
                  <p className="text-sm text-text-primary dark:text-white/70 mt-2 bg-black/[0.03] dark:bg-white/[0.04] rounded-lg px-3 py-2">
                    {it.note}
                  </p>
                )}
              </div>
              <button
                onClick={() => onOpenInArchive(it.document_title)}
                className="shrink-0 flex items-center gap-1.5 text-xs text-accent hover:text-primary dark:hover:text-white px-2.5 py-1.5 rounded-lg hover:bg-accent/5 transition-colors"
                title="ابحث عن هذه الوثيقة في الأرشيف"
              >
                <IconSearch size={13} /> في الأرشيف
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
