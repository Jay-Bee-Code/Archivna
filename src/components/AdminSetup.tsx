import Logo from "./Logo";
import { useState } from "react";
import { api, errorMessage } from "../lib/api";

export default function AdminSetup({ onCreated }: { onCreated: () => void }) {
  const [username, setUsername] = useState("");
  const [fullName, setFullName] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      await api.createAdmin(username.trim(), password, fullName.trim() || null);
      onCreated();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-bg-light dark:bg-bg-dark px-4">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm bg-white dark:bg-white/[0.04] border border-border-light dark:border-white/10 rounded-2xl p-7 flex flex-col gap-4 shadow-sm dark:shadow-none"
      >
        <div>
          <div className="flex justify-center mb-1"><Logo size={36} /></div>
        <h1 className="text-lg font-bold text-primary dark:text-white">إنشاء حساب المدير</h1>
          <p className="text-xs text-text-secondary mt-1">
            أول حساب في هذه الخزنة يكون تلقائيًا بدور المدير (admin)، ويمكنه لاحقًا إنشاء بقية الحسابات.
          </p>
        </div>

        <input
          value={fullName}
          onChange={(e) => setFullName(e.target.value)}
          placeholder="الاسم الكامل (اختياري)"
          className="px-3.5 py-2.5 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white placeholder:text-text-secondary/60 dark:placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-accent transition-shadow"
        />
        <input
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          placeholder="اسم المستخدم"
          autoFocus
          className="px-3.5 py-2.5 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white placeholder:text-text-secondary/60 dark:placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-accent transition-shadow"
        />
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="كلمة المرور (8 أحرف على الأقل)"
          className="px-3.5 py-2.5 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white placeholder:text-text-secondary/60 dark:placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-accent transition-shadow"
        />

        {error && <p className="text-sm text-danger">{error}</p>}

        <button
          type="submit"
          disabled={loading || !username.trim() || password.length < 8}
          className="py-2.5 rounded-lg bg-primary text-white text-sm font-medium hover:bg-accent shadow-sm hover:shadow transition-all disabled:opacity-40 disabled:shadow-none"
        >
          {loading ? "جارٍ الإنشاء…" : "إنشاء الحساب"}
        </button>
      </form>
    </div>
  );
}
