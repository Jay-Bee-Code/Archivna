import { useEffect, useState } from "react";
import { api, Document, DisposalRecord, errorMessage } from "../lib/api";
import { IconScale, IconRecycle, IconClose } from "./icons/Icon";

function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString("ar-DZ", { year: "numeric", month: "long", day: "numeric" });
}

export default function RetentionPanel({ onClose }: { onClose: () => void }) {
  const [tab, setTab] = useState<"candidates" | "log">("candidates");
  const [candidates, setCandidates] = useState<Document[]>([]);
  const [log, setLog] = useState<DisposalRecord[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [reasonFor, setReasonFor] = useState<string | null>(null);
  const [reason, setReason] = useState("");

  function refresh() {
    api.listDisposalCandidates().then(setCandidates).catch((e) => setError(errorMessage(e)));
    api.listDisposalLog().then(setLog).catch(() => {});
  }
  useEffect(refresh, []);

  async function handleLegalHold(id: string, hold: boolean) {
    setBusyId(id);
    try {
      await api.setLegalHold(id, hold);
      refresh();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusyId(null);
    }
  }

  async function handleDispose(id: string) {
    setBusyId(id);
    try {
      await api.disposeDocument(id, reason.trim() || null);
      setReasonFor(null);
      setReason("");
      refresh();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusyId(null);
    }
  }

  const tabClass = (active: boolean) =>
    `text-sm px-3 py-1.5 rounded-lg transition-colors ${
      active ? "bg-primary text-white" : "text-text-secondary dark:text-white/50 hover:bg-black/5 dark:hover:bg-white/5"
    }`;

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-40 px-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-2xl bg-white dark:bg-[#132033] border border-border-light dark:border-white/10 rounded-2xl p-6 flex flex-col gap-4 max-h-[85vh] overflow-y-auto"
      >
        <div className="flex items-center justify-between">
          <h2 className="flex items-center gap-2 text-base font-bold text-primary dark:text-white">
            <IconRecycle size={18} /> الاحتفاظ والإتلاف
          </h2>
          <button
            onClick={onClose}
            className="text-text-secondary dark:text-white/50 hover:text-danger hover:bg-danger/5 p-1.5 rounded-lg transition-colors"
          >
            <IconClose size={16} />
          </button>
        </div>

        <div className="flex gap-1.5">
          <button onClick={() => setTab("candidates")} className={tabClass(tab === "candidates")}>
            مؤهَّلة للإتلاف ({candidates.length})
          </button>
          <button onClick={() => setTab("log")} className={tabClass(tab === "log")}>
            محضر الإتلاف ({log.length})
          </button>
        </div>

        {error && <p className="text-sm text-danger bg-danger/10 rounded-md p-2">{error}</p>}

        {tab === "candidates" && (
          <div className="flex flex-col gap-2">
            {candidates.length === 0 && (
              <p className="text-sm text-text-secondary dark:text-white/40">
                لا توجد وثائق مؤهَّلة للإتلاف حاليًا — إما لم تنتهِ مدة احتفاظ أي وثيقة بعد، أو لم تُعيَّن مدة احتفاظ لأي نوع.
              </p>
            )}
            {candidates.map((d) => (
              <div key={d.id} className="border border-border-light dark:border-white/10 rounded-lg p-3 flex flex-col gap-2">
                <div className="flex items-center justify-between gap-2">
                  <div className="min-w-0">
                    <p className="text-sm font-medium text-text-primary dark:text-white truncate">{d.title}</p>
                    <p className="text-[11px] text-text-secondary dark:text-white/40">
                      رُفعت في {formatDate(d.created_at)}
                      {d.registry_number && ` • ${d.registry_number}`}
                    </p>
                  </div>
                  {d.legal_hold && (
                    <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-warning/15 text-warning shrink-0">
                      <IconScale size={11} /> مجمَّدة
                    </span>
                  )}
                </div>

                <div className="flex gap-2">
                  <button
                    onClick={() => handleLegalHold(d.id, !d.legal_hold)}
                    disabled={busyId === d.id}
                    className="flex-1 text-xs py-1.5 rounded-lg border border-border-light dark:border-white/10 text-text-secondary dark:text-white/60 hover:bg-black/5 dark:hover:bg-white/5 disabled:opacity-40"
                  >
                    {d.legal_hold ? "رفع التجميد" : "تجميد قانوني"}
                  </button>
                  {!d.legal_hold && (
                    reasonFor === d.id ? (
                      <div className="flex-1 flex gap-1.5">
                        <input
                          value={reason}
                          onChange={(e) => setReason(e.target.value)}
                          placeholder="سبب الإتلاف (اختياري)"
                          className="flex-1 min-w-0 text-xs px-2 py-1.5 rounded-lg border border-danger/40 bg-white dark:bg-white/5 dark:text-white"
                          autoFocus
                        />
                        <button
                          onClick={() => handleDispose(d.id)}
                          disabled={busyId === d.id}
                          className="shrink-0 text-xs px-3 rounded-lg bg-danger text-white disabled:opacity-40"
                        >
                          تأكيد الإتلاف
                        </button>
                      </div>
                    ) : (
                      <button
                        onClick={() => setReasonFor(d.id)}
                        className="flex-1 text-xs py-1.5 rounded-lg bg-danger/10 text-danger hover:bg-danger/20"
                      >
                        إتلاف نهائي
                      </button>
                    )
                  )}
                </div>
              </div>
            ))}
          </div>
        )}

        {tab === "log" && (
          <div className="flex flex-col gap-2">
            {log.length === 0 && (
              <p className="text-sm text-text-secondary dark:text-white/40">لا سجلات إتلاف بعد.</p>
            )}
            {log.map((r) => (
              <div key={r.id} className="border border-border-light dark:border-white/10 rounded-lg p-3">
                <p className="text-sm text-text-primary dark:text-white">{r.title}</p>
                <p className="text-[11px] text-text-secondary dark:text-white/40">
                  أُتلفت في {formatDate(r.disposed_at)}
                  {r.registry_number && ` • ${r.registry_number}`}
                  {r.reason && ` • السبب: ${r.reason}`}
                </p>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
