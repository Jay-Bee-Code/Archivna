import { useEffect, useState, useCallback } from "react";
import { api, SyncStatus, errorMessage } from "../lib/api";

function timeAgo(ts: number | null): string {
  if (!ts) return "أبدًا";
  const diff = Math.floor(Date.now() / 1000) - ts;
  if (diff < 10) return "الآن";
  if (diff < 60) return `قبل ${diff} ثانية`;
  if (diff < 3600) return `قبل ${Math.floor(diff / 60)} دقيقة`;
  return `قبل ${Math.floor(diff / 3600)} ساعة`;
}

export default function NetworkStatus() {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [open, setOpen] = useState(false);
  const [syncing, setSyncing] = useState(false);

  const refresh = useCallback(() => {
    api.syncStatus().then(setStatus).catch(() => {});
  }, []);

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 5000);
    return () => clearInterval(t);
  }, [refresh]);

  async function handleSyncNow() {
    setSyncing(true);
    try {
      await api.syncNow();
    } catch (e) {
      // يُعرض في الحالة على أي حال عبر last_error لكل جهاز
      console.error(errorMessage(e));
    } finally {
      setTimeout(() => setSyncing(false), 1000);
      setTimeout(refresh, 1500);
    }
  }

  if (!status) return null;

  const dotColor = !status.configured
    ? "bg-text-secondary/40"
    : status.peers.length > 0
      ? "bg-success"
      : "bg-warning";

  const label = !status.configured
    ? "المزامنة غير مُفعَّلة"
    : status.peers.length > 0
      ? `متصل بـ ${status.peers.length} جهاز`
      : "لا أجهزة على الشبكة";

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        className="flex items-center gap-2 text-xs text-text-secondary dark:text-white/50 hover:text-text-primary dark:hover:text-white transition-colors"
      >
        <span className={`w-2 h-2 rounded-full ${dotColor}`} />
        {label}
      </button>

      {open && (
        <div className="absolute bottom-full start-0 mb-2 w-72 bg-white dark:bg-[#132033] border border-border-light dark:border-white/10 rounded-xl shadow-lg dark:shadow-black/30 p-3.5 z-30 text-start">
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-xs font-semibold text-text-secondary dark:text-white/50">
              أجهزة الشبكة المحلية
            </h3>
            {status.configured && (
              <button
                onClick={handleSyncNow}
                disabled={syncing}
                className="text-[11px] text-accent hover:underline disabled:opacity-50"
              >
                {syncing ? "جارٍ…" : "مزامنة الآن"}
              </button>
            )}
          </div>

          {!status.configured ? (
            <p className="text-xs text-text-secondary dark:text-white/50">
              لم يُفعَّل مفتاح مزامنة لهذه الخزنة بعد.
            </p>
          ) : status.peers.length === 0 ? (
            <p className="text-xs text-text-secondary dark:text-white/50">
              لم يُكتشف أي جهاز آخر على هذه الشبكة المحلية حتى الآن.
            </p>
          ) : (
            <ul className="flex flex-col gap-2">
              {status.peers.map((p) => (
                <li key={p.node_id} className="text-xs">
                  <div className="flex items-center justify-between">
                    <span className="text-text-primary dark:text-white">{p.name}</span>
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${
                        p.last_error ? "bg-danger" : "bg-success"
                      }`}
                    />
                  </div>
                  <div className="text-text-secondary dark:text-white/40 text-[11px]">
                    آخر مزامنة: {timeAgo(p.last_sync)}
                  </div>
                  {p.last_error && (
                    <div className="text-danger text-[11px] truncate" title={p.last_error}>
                      {p.last_error}
                    </div>
                  )}
                </li>
              ))}
            </ul>
          )}

          {status.node_name && (
            <p className="text-[10px] text-text-secondary dark:text-white/30 mt-3 pt-2 border-t border-border-light dark:border-white/10">
              اسم هذا الجهاز: {status.node_name}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
