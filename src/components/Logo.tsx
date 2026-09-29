export default function Logo({ size = 28 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className="shrink-0"
    >
      {/* درع بسيط يرمز للحماية والرسمية */}
      <path
        d="M16 2L28 6.5V15C28 22.5 22.8 27.8 16 30C9.2 27.8 4 22.5 4 15V6.5L16 2Z"
        className="fill-primary dark:fill-accent"
      />
      {/* ملف/وثيقة داخل الدرع */}
      <rect x="11" y="10" width="10" height="13" rx="1.2" className="fill-white/90" />
      <rect x="13" y="13" width="6" height="1.4" rx="0.7" className="fill-primary dark:fill-accent" />
      <rect x="13" y="16" width="6" height="1.4" rx="0.7" className="fill-primary dark:fill-accent" />
      <rect x="13" y="19" width="4" height="1.4" rx="0.7" className="fill-primary dark:fill-accent" />
    </svg>
  );
}
