/** @type {import('tailwindcss').Config} */
// كل لون يشير إلى متغيّر CSS من styles.css، فتعمل الألوان في الوضعين من مصدر واحد.
const v = (name) => `var(${name})`;

export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: ["class", '[data-theme="dark"]'],
  theme: {
    extend: {
      colors: {
        page: v("--surface-page"),
        raised: v("--surface-raised"),
        sunken: v("--surface-sunken"),
        hover: v("--surface-hover"),
        active: v("--surface-active"),
        line: v("--border-subtle"),
        "line-strong": v("--border-strong"),
        // أسماء الإصدار القديم، مربوطة بالتوكنات الجديدة كي لا تفقد 209 استخدامًا ألوانها
        // أثناء الهجرة التدريجية. تُزال بعد أن تنتقل كل المكونات إلى التوكنات الدلالية.
        primary: v("--accent"),
        "bg-light": v("--surface-page"),
        "bg-dark": v("--surface-page"),
        "text-primary": v("--text-strong"),
        "text-secondary": v("--text-muted"),
        "border-light": v("--border-subtle"),
        ink: v("--text-strong"),
        body: v("--text-body"),
        muted: v("--text-muted"),
        faint: v("--text-faint"),
        accent: v("--accent"),
        "accent-strong": v("--accent-strong"),
        "accent-soft": v("--accent-soft"),
        "on-accent": v("--on-accent"),
        warning: v("--sem-warning"),
        "warning-soft": v("--sem-warning-soft"),
        danger: v("--sem-danger"),
        "danger-soft": v("--sem-danger-soft"),
        success: v("--sem-success"),
        "success-soft": v("--sem-success-soft"),
      },
      fontSize: {
        // مقياس من أربع خطوات فقط. لا أحجام عشوائية في المكونات.
        xs: ["12px", { lineHeight: "16px" }],
        sm: ["13px", { lineHeight: "20px" }],
        base: ["14px", { lineHeight: "22px" }],
        lg: ["18px", { lineHeight: "26px", letterSpacing: "-0.01em" }],
        xl: ["24px", { lineHeight: "32px", letterSpacing: "-0.02em" }],
      },
      borderRadius: {
        sm: "var(--radius-sm)",
        md: "var(--radius-md)",
        lg: "var(--radius-lg)",
      },
      boxShadow: {
        sm: "var(--shadow-sm)",
        md: "var(--shadow-md)",
      },
      fontFamily: {
        sans: ['"IBM Plex Sans Arabic"', '"Noto Kufi Arabic"', "system-ui", "sans-serif"],
      },
    },
  },
  plugins: [],
};
