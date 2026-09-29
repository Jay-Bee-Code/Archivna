export default function EmptyState({
  title,
  hint,
}: {
  title: string;
  hint?: string;
}) {
  return (
    <div className="flex flex-col items-center justify-center text-center py-20 px-6">
      <svg
        width="56"
        height="56"
        viewBox="0 0 56 56"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        className="mb-4 opacity-40"
      >
        <rect x="10" y="14" width="36" height="30" rx="3" className="stroke-text-secondary dark:stroke-white/40" strokeWidth="2" />
        <path d="M10 22H46" className="stroke-text-secondary dark:stroke-white/40" strokeWidth="2" />
        <circle cx="28" cy="33" r="6" className="stroke-text-secondary dark:stroke-white/40" strokeWidth="2" />
        <path d="M25 33H31M28 30V36" className="stroke-text-secondary dark:stroke-white/40" strokeWidth="2" strokeLinecap="round" />
      </svg>
      <p className="text-sm font-medium text-text-primary dark:text-white/80">{title}</p>
      {hint && <p className="text-xs text-text-secondary dark:text-white/40 mt-1 max-w-xs">{hint}</p>}
    </div>
  );
}
