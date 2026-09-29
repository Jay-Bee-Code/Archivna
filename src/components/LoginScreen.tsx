import { useState } from "react";
import { api, errorMessage, UserPublic } from "../lib/api";

export default function LoginScreen({ onLoggedIn }: { onLoggedIn: (user: UserPublic) => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const user = await api.login(username.trim(), password);
      onLoggedIn(user);
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
        className="w-full max-w-sm bg-white dark:bg-white/5 border border-border-light dark:border-white/10 rounded-lg p-6 flex flex-col gap-4"
      >
        <h1 className="text-lg font-bold text-primary dark:text-white">تسجيل الدخول</h1>

        <input
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          placeholder="اسم المستخدم"
          autoFocus
          className="px-3 py-2 rounded-md border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent"
        />
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="كلمة المرور"
          className="px-3 py-2 rounded-md border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-2 focus:ring-accent"
        />

        {error && <p className="text-sm text-danger">{error}</p>}

        <button
          type="submit"
          disabled={loading || !username.trim() || !password}
          className="py-2 rounded-md bg-primary text-white text-sm hover:bg-accent transition-colors disabled:opacity-40"
        >
          {loading ? "جارٍ الدخول…" : "دخول"}
        </button>
      </form>
    </div>
  );
}
