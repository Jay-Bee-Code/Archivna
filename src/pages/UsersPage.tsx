import { useEffect, useState } from "react";
import {
  api,
  Department,
  UserPublic,
  Role,
  CONFIDENTIALITY_LABELS,
  errorMessage,
} from "../lib/api";
import { flattenDeptTree, deptIndent } from "../lib/deptTree";
import { IconUsers, IconPlus } from "../components/icons/Icon";

const ROLE_LABELS: Record<string, string> = {
  admin: "مدير",
  archivist: "موظف أرشفة",
  reviewer: "مراجع",
  viewer: "قراءة فقط",
};

export default function UsersPage() {
  const [users, setUsers] = useState<UserPublic[]>([]);
  const [departments, setDepartments] = useState<Department[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const [username, setUsername] = useState("");
  const [fullName, setFullName] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Role>("archivist");
  const [departmentId, setDepartmentId] = useState<string | null>(null);
  const [clearance, setClearance] = useState(1);
  const [loading, setLoading] = useState(false);

  function refresh() {
    api.listUsers().then(setUsers).catch((e) => setError(errorMessage(e)));
    api.listDepartments().then(setDepartments).catch(() => {});
  }
  useEffect(refresh, []);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setSuccess(null);
    setLoading(true);
    try {
      await api.createUser({
        username: username.trim(),
        password,
        fullName: fullName.trim() || null,
        role,
        departmentId,
        clearanceLevel: clearance,
      });
      setSuccess(`تم إنشاء الحساب "${username.trim()}" بنجاح`);
      setUsername("");
      setFullName("");
      setPassword("");
      refresh();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  const deptName = (id: string | null) =>
    id ? departments.find((d) => d.id === id)?.name_ar ?? "—" : "مركزي (كل الأقسام)";

  const fieldClass =
    "w-full px-3 py-2 rounded-lg border border-border-light dark:border-white/10 bg-white dark:bg-white/5 dark:text-white text-sm focus:outline-none focus:ring-2 focus:ring-accent";

  return (
    <div className="max-w-5xl mx-auto p-6 md:p-8 flex flex-col gap-5">
      <h2 className="flex items-center gap-2.5 text-xl font-bold text-primary dark:text-white">
        <IconUsers size={22} /> المستخدمون
      </h2>

      <div className="grid grid-cols-1 lg:grid-cols-5 gap-5">
        <section className="lg:col-span-3 bg-white dark:bg-white/[0.03] border border-border-light dark:border-white/10 rounded-2xl p-5">
          <h3 className="text-xs font-semibold text-text-secondary dark:text-white/40 uppercase tracking-wide mb-3">
            الحسابات النشطة ({users.length})
          </h3>
          <div className="flex flex-col gap-1.5">
            {users.map((u) => (
              <div
                key={u.id}
                className="flex items-center justify-between gap-3 px-3 py-2.5 rounded-lg bg-black/[0.02] dark:bg-white/[0.03]"
              >
                <div className="min-w-0">
                  <p className="text-sm text-text-primary dark:text-white truncate">
                    {u.full_name || u.username}
                    {u.full_name && (
                      <span className="text-text-secondary dark:text-white/40 text-xs"> @{u.username}</span>
                    )}
                  </p>
                  <p className="text-[11px] text-text-secondary dark:text-white/40">
                    {deptName(u.department_id)} • اطّلاع: {CONFIDENTIALITY_LABELS[u.clearance_level] ?? u.clearance_level}
                  </p>
                </div>
                <span className="shrink-0 text-[11px] px-2 py-0.5 rounded-full bg-primary/10 dark:bg-accent/20 text-primary dark:text-accent">
                  {ROLE_LABELS[u.role] || u.role}
                </span>
              </div>
            ))}
          </div>
        </section>

        <section className="lg:col-span-2 bg-white dark:bg-white/[0.03] border border-border-light dark:border-white/10 rounded-2xl p-5">
          <h3 className="text-xs font-semibold text-text-secondary dark:text-white/40 uppercase tracking-wide mb-3">
            إضافة مستخدم
          </h3>
          <form onSubmit={handleSubmit} className="flex flex-col gap-3">
            <input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="اسم المستخدم" className={fieldClass} />
            <input value={fullName} onChange={(e) => setFullName(e.target.value)} placeholder="الاسم الكامل (اختياري)" className={fieldClass} />
            <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="كلمة المرور (8 أحرف على الأقل)" className={fieldClass} />

            <label className="flex flex-col gap-1">
              <span className="text-xs text-text-secondary dark:text-white/50">الدور</span>
              <select value={role} onChange={(e) => setRole(e.target.value as Role)} className={fieldClass}>
                {Object.entries(ROLE_LABELS).map(([k, label]) => (
                  <option key={k} value={k}>{label}</option>
                ))}
              </select>
            </label>

            <label className="flex flex-col gap-1">
              <span className="text-xs text-text-secondary dark:text-white/50">القسم</span>
              <select value={departmentId ?? ""} onChange={(e) => setDepartmentId(e.target.value || null)} className={fieldClass}>
                <option value="">مركزي (يرى كل الأقسام)</option>
                {flattenDeptTree(departments).map(({ dept, depth }) => (
                  <option key={dept.id} value={dept.id}>{deptIndent(depth)}{dept.name_ar}</option>
                ))}
              </select>
            </label>

            <label className="flex flex-col gap-1">
              <span className="text-xs text-text-secondary dark:text-white/50">مستوى الاطّلاع (أعلى سرية يستطيع رؤيتها)</span>
              <select value={clearance} onChange={(e) => setClearance(Number(e.target.value))} className={fieldClass}>
                {Object.entries(CONFIDENTIALITY_LABELS).map(([lvl, label]) => (
                  <option key={lvl} value={lvl}>{label}</option>
                ))}
              </select>
            </label>

            {error && <p className="text-sm text-danger">{error}</p>}
            {success && <p className="text-sm text-success">{success}</p>}

            <button
              type="submit"
              disabled={loading || !username.trim() || password.length < 8}
              className="flex items-center justify-center gap-2 py-2.5 rounded-lg bg-primary text-white text-sm font-medium hover:bg-accent shadow-sm transition-all disabled:opacity-40"
            >
              <IconPlus size={14} /> {loading ? "جارٍ الإنشاء…" : "إنشاء الحساب"}
            </button>
          </form>
        </section>
      </div>
    </div>
  );
}
