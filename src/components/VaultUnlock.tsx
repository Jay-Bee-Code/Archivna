import { useState, useEffect } from "react";
import { api, errorMessage } from "../lib/api";

export default function VaultUnlock({
  onUnlocked,
}: {
  onUnlocked: (hasAdmin: boolean) => void;
}) {
  const [isFirstRun, setIsFirstRun] = useState<boolean | null>(null);
  const [passphrase, setPassphrase] = useState("");
  const [confirm, setConfirm] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.vaultExists().then((exists) => setIsFirstRun(!exists));
  }, []);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);

    if (isFirstRun && passphrase !== confirm) {
      setError("العبارتان غير متطابقتين");
      return;
    }
    if (passphrase.length < 8) {
      setError("العبارة السرية يجب أن تكون 8 أحرف على الأقل");
      return;
    }

    setLoading(true);
    try {
      const result = await api.unlockVault(passphrase);
      onUnlocked(result.has_admin);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  if (isFirstRun === null) {
    return <div className="min-h-screen flex items-center justify-center text-text-secondary">جارٍ التحقق…</div>;
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-bg-light dark:bg-bg-dark px-4">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm bg-white dark:bg-white/5 border border-border-light dark:border-white/10 rounded-lg p-6 flex flex-col gap-4"
      >
        <div>
          <h1 className="text-lg font-bold text-primary dark:text-white">
            {isFirstRun ? "إنشاء خزنة جديدة" : "فتح الخزنة"}
          </h1>
          <p className="text-xs text-text-secondary mt-1">
            {isFirstRun
              ? "هذه العبارة تُشفِّر كامل الأرشيف على هذا الجهاز — احفظها في مكان آمن، فقدانها يعني فقدان الوصول للأرشيف نهائيًا."
              : "أدخل العبارة السرية لفتح أرشيف هذا الجهاز."}
          </p>
        </div>

        <input
          type="password"
          value={passphrase}
          onChange={(e) => setPassphrase(e.target.value)}
          placeholder="العبارة السرية"
          autoFocus
          className="px-3 py-2 rounded-md border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent"
        />

        {isFirstRun && (
          <input
            type="password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            placeholder="تأكيد العبارة السرية"
            className="px-3 py-2 rounded-md border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent"
          />
        )}

        {error && <p className="text-sm text-danger">{error}</p>}

        <button
          type="submit"
          disabled={loading || !passphrase}
          className="py-2 rounded-md bg-primary text-white text-sm hover:bg-accent transition-colors disabled:opacity-40"
        >
          {loading ? "جارٍ الفتح…" : isFirstRun ? "إنشاء الخزنة" : "فتح"}
        </button>
      </form>
    </div>
  );
}
