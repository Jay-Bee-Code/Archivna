import { useEffect, useState } from "react";
import { api, UserPublic, RoutingEntry, errorMessage } from "../lib/api";
import { IconRoute, IconClose } from "./icons/Icon";

function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString("ar-DZ", { year: "numeric", month: "short", day: "numeric" });
}

export default function RoutingModal({
  documentId,
  title,
  onClose,
}: {
  documentId: string;
  title: string;
  onClose: () => void;
}) {
  const [users, setUsers] = useState<UserPublic[]>([]);
  const [history, setHistory] = useState<RoutingEntry[]>([]);
  const [toUserId, setToUserId] = useState("");
  const [note, setNote] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function refreshHistory() {
    api.listDocumentRouting(documentId).then(setHistory).catch(() => {});
  }

  useEffect(() => {
    api.listUsers().then(setUsers).catch(() => {});
    refreshHistory();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [documentId]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!toUserId) return;
    setLoading(true);
    setError(null);
    try {
      await api.routeDocument(documentId, toUserId, note.trim() || null);
      setNote("");
      setToUserId("");
      refreshHistory();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50 px-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-sm bg-white dark:bg-[#132033] border border-border-light dark:border-white/10 rounded-2xl p-6 flex flex-col gap-3 max-h-[85vh] overflow-y-auto"
      >
        <div className="flex items-center justify-between">
          <h2 className="flex items-center gap-1.5 text-sm font-bold text-primary dark:text-white truncate min-w-0">
            <IconRoute size={15} className="shrink-0" /> <span className="truncate">إحالة: {title}</span>
          </h2>
          <button
            onClick={onClose}
            className="text-text-secondary dark:text-white/50 hover:text-danger hover:bg-danger/5 p-1 rounded-lg shrink-0 transition-colors"
          >
            <IconClose size={15} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-2">
          <select
            value={toUserId}
            onChange={(e) => setToUserId(e.target.value)}
            className="text-sm px-3 py-2 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent"
          >
            <option value="">اختر مستخدمًا…</option>
            {users.map((u) => (
              <option key={u.id} value={u.id}>{u.full_name || u.username}</option>
            ))}
          </select>
          <input
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="ملاحظة (اختياري)"
            className="text-sm px-3 py-2 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent"
          />
          {error && <p className="text-xs text-danger">{error}</p>}
          <button
            type="submit"
            disabled={loading || !toUserId}
            className="py-2 rounded-lg bg-primary text-white text-sm font-medium hover:bg-accent disabled:opacity-40"
          >
            {loading ? "جارٍ الإحالة…" : "إحالة"}
          </button>
        </form>

        <div className="border-t border-border-light dark:border-white/10 pt-3 flex flex-col gap-2">
          <h3 className="text-xs font-semibold text-text-secondary dark:text-white/40 uppercase tracking-wide">
            سجل الإحالات
          </h3>
          {history.length === 0 && (
            <p className="text-xs text-text-secondary dark:text-white/40">لم تُحَل هذه الوثيقة بعد.</p>
          )}
          {history.map((h) => (
            <div key={h.id} className="text-xs bg-black/[0.02] dark:bg-white/[0.03] rounded-lg p-2">
              <p className="text-text-primary dark:text-white">
                إلى {h.routed_to_username || h.routed_to_user_id} — {formatDate(h.routed_at)}
              </p>
              {h.note && <p className="text-text-secondary dark:text-white/40 mt-0.5">{h.note}</p>}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
