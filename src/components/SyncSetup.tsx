import { useState } from "react";
import { api, errorMessage } from "../lib/api";

/**
 * تُعرض مرة واحدة بعد إنشاء حساب المدير الأول. تحدد ما إذا كانت هذه الخزنة
 * ستزامن مع أجهزة أخرى على نفس الشبكة المحلية عبر "مفتاح مزامنة" مشترك
 * (يختلف عن العبارة السرية لكل جهاز — انظر القسم 10.2/10.3 في README).
 */
export default function SyncSetup({ onDone }: { onDone: () => void }) {
  const [mode, setMode] = useState<"choose" | "form">("choose");
  const [passphrase, setPassphrase] = useState("");
  const [confirm, setConfirm] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (passphrase !== confirm) {
      setError("المفتاحان غير متطابقين");
      return;
    }
    setLoading(true);
    try {
      await api.setSyncKey(passphrase);
      onDone();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-bg-light dark:bg-bg-dark px-4">
      <div className="w-full max-w-sm bg-white dark:bg-white/5 border border-border-light dark:border-white/10 rounded-lg p-6 flex flex-col gap-4">
        <div>
          <h1 className="text-lg font-bold text-primary dark:text-white">مزامنة الشبكة المحلية</h1>
          <p className="text-xs text-text-secondary mt-1">
            إن كان لديك أجهزة أخرى في نفس المصلحة تريد مشاركة الأرشيف معها تلقائيًا عبر الشبكة
            المحلية، فعّل المزامنة الآن بمفتاح مشترك. يمكن تفعيلها لاحقًا من إعدادات المدير.
          </p>
        </div>

        {mode === "choose" ? (
          <div className="flex flex-col gap-2">
            <button
              onClick={() => setMode("form")}
              className="py-2 rounded-md bg-primary text-white text-sm hover:bg-accent transition-colors"
            >
              تفعيل المزامنة الآن
            </button>
            <button
              onClick={onDone}
              className="py-2 rounded-md border border-border-light dark:border-white/10 text-text-secondary text-sm hover:bg-black/5 dark:hover:bg-white/5"
            >
              تخطّي — العمل على هذا الجهاز فقط
            </button>
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="flex flex-col gap-3">
            <p className="text-[11px] text-text-secondary">
              أدخل نفس هذا المفتاح على كل جهاز آخر تريد ربطه بهذا الأرشيف. هذا المفتاح مستقل عن
              عبارة فتح الخزنة الخاصة بكل جهاز.
            </p>
            <input
              type="password"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
              placeholder="مفتاح المزامنة المشترك"
              autoFocus
              className="px-3 py-2 rounded-md border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent"
            />
            <input
              type="password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              placeholder="تأكيد مفتاح المزامنة"
              className="px-3 py-2 rounded-md border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent"
            />

            {error && <p className="text-sm text-danger">{error}</p>}

            <button
              type="submit"
              disabled={loading || passphrase.length < 8}
              className="py-2 rounded-md bg-primary text-white text-sm hover:bg-accent transition-colors disabled:opacity-40"
            >
              {loading ? "جارٍ التفعيل…" : "تفعيل"}
            </button>
            <button
              type="button"
              onClick={() => setMode("choose")}
              className="text-xs text-text-secondary hover:underline"
            >
              رجوع
            </button>
          </form>
        )}
      </div>
    </div>
  );
}
