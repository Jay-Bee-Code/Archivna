import { useState } from "react";
import { Category } from "../lib/api";

export default function CategorySidebar({
  categories,
  activeCategory,
  onSelect,
  onAddCategory,
}: {
  categories: Category[];
  activeCategory: string | null;
  onSelect: (id: string | null) => void;
  onAddCategory: (name: string) => Promise<void>;
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
      <h2 className="text-xs font-semibold text-text-secondary dark:text-white/40 mb-2 px-1">
        الفئات
      </h2>

      <button
        onClick={() => onSelect(null)}
        className={`text-start px-3 py-2 rounded-md text-sm transition-colors ${
          activeCategory === null
            ? "bg-primary text-white"
            : "text-text-primary dark:text-white/80 hover:bg-black/5 dark:hover:bg-white/5"
        }`}
      >
        كل الوثائق
      </button>

      {categories.map((cat) => (
        <button
          key={cat.id}
          onClick={() => onSelect(cat.id)}
          className={`text-start px-3 py-2 rounded-md text-sm flex items-center justify-between transition-colors ${
            activeCategory === cat.id
              ? "bg-primary text-white"
              : "text-text-primary dark:text-white/80 hover:bg-black/5 dark:hover:bg-white/5"
          }`}
        >
          <span className="truncate">{cat.name}</span>
          <span
            className={`text-[11px] ${
              activeCategory === cat.id ? "text-white/70" : "text-text-secondary dark:text-white/40"
            }`}
          >
            {cat.document_count}
          </span>
        </button>
      ))}

      <form onSubmit={handleAdd} className="mt-3 flex gap-1 px-1">
        <input
          type="text"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          placeholder="فئة جديدة…"
          className="flex-1 min-w-0 text-xs px-2 py-1.5 rounded-md border border-border-light dark:border-white/10
            bg-white dark:bg-white/5 dark:text-white focus:outline-none focus:ring-1 focus:ring-accent"
        />
        <button
          type="submit"
          disabled={adding || !newName.trim()}
          className="text-xs px-2 py-1.5 rounded-md bg-accent text-white disabled:opacity-40 shrink-0"
        >
          +
        </button>
      </form>
    </div>
  );
}
