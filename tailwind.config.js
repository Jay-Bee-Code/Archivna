/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: "media",
  theme: {
    extend: {
      colors: {
        primary: "#1A3A5C",
        accent: "#3B6EA5",
        success: "#2E7D5B",
        warning: "#B8860B",
        danger: "#B23A3A",
        "bg-light": "#F7F8FA",
        "bg-dark": "#0F1B2B",
        "text-primary": "#1E2A38",
        "text-secondary": "#5A6B7B",
        "border-light": "#DDE3EA",
      },
      fontFamily: {
        arabic: ["IBM Plex Sans Arabic", "Noto Kufi Arabic", "sans-serif"],
        latin: ["Inter", "IBM Plex Sans", "sans-serif"],
      },
    },
  },
  plugins: [],
};
