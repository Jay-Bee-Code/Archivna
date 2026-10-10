import { useState } from "react";
import { Category } from "../lib/api";
import { IconPlus, IconTrash } from "./icons/Icon";

export default function CategorySidebar({
  categories,
  activeCategory,
  onSelect,
  onAddCategory,
  onDeleteCategory,
}: {
  categories: Category[];
  activeCategory: string | null;
  onSelect: (id: string | null) => void;
  onAddCategory: (name: string) => Promise<void>;
  /** غير معرَّف = لا صلاحية حذف (الحذف للمدير فقط في الـ backend) */
  onDeleteCategory?: (id: string) => void;
}) {
  const [newName, setNewName] = useState("");
  const [adding, setAdding] = useState(false);

  async function handleAdd(e: React.FormEvent) {
    e.preventDefault();
    if (!newName.trim()) return;
    setAdding(true);
    try {
      await onAddCategory(newName.trim());
      setNewName("");
    } finally {
      setAdding(false);
    }
  }

  return (
    <div className="flex flex-col gap-1">
      <h2 className="text-[11px] font-semibold text-text-secondary dark:text-white/40 mb-1 px-1 uppercase tracking-wide">
        الفئات
      </h2>

      <button
        onClick={() => onSelect(null)}
        className={`text-start px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
          activeCategory === null
            ? "bg-primary dark:bg-accent/25 text-white dark:text-white"
            : "text-text-primary dark:text-white/70 hover:bg-black/5 dark:hover:bg-white/5"
        }`}
      >
        كل الوثائق
      </button>

      {categories.map((cat) => (
        // الصف div وليس button: زر الحذف لا يصحّ تعشيشه داخل زر الاختيار (HTML غير صالح)
        <div key={cat.id} className="group flex items-stretch gap-0.5">
          <button
            onClick={() => onSelect(cat.id)}
            className={`flex-1 min-w-0 text-start px-3 py-2 rounded-lg text-sm flex items-center justify-between transition-colors ${
              activeCategory === cat.id
                ? "bg-primary dark:bg-accent/25 text-white dark:text-white"
                : "text-text-primary dark:text-white/70 hover:bg-black/5 dark:hover:bg-white/5"
            }`}
          >
            <span className="truncate">{cat.name}</span>
            <span
              className={`text-[11px] tabular-nums ${
                activeCategory === cat.id ? "text-white/70" : "text-text-secondary dark:text-white/30"
              }`}
            >
              {cat.document_count}
            </span>
          </button>
          {onDeleteCategory && (
            <button
              onClick={() => onDeleteCategory(cat.id)}
              title="حذف الفئة (تُرفض إن احتوت وثائق)"
              className="shrink-0 w-0 group-hover:w-7 overflow-hidden flex items-center justify-center rounded-lg text-text-secondary/50 hover:text-danger hover:bg-danger/5 transition-all"
            >
              <IconTrash size={13} />
            </button>
          )}
        </div>
      ))}

      <form onSubmit={handleAdd} className="mt-3 flex gap-1 px-1">
        <input
          type="text"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          placeholder="فئة جديدة…"
          className="flex-1 min-w-0 text-xs px-2.5 py-1.5 rounded-lg border border-border-light dark:border-white/10
            bg-white dark:bg-white/5 dark:text-white placeholder:text-text-secondary/60 dark:placeholder:text-white/30
            focus:outline-none focus:ring-1 focus:ring-accent"
        />
        <button
          type="submit"
          disabled={adding || !newName.trim()}
          className="px-2.5 py-1.5 rounded-lg bg-accent text-white disabled:opacity-40 shrink-0 hover:bg-primary transition-colors flex items-center justify-center"
          title="إضافة"
        >
          <IconPlus size={12} />
        </button>
      </form>
    </div>
  );
}
