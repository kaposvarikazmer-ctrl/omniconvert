/** @type {import('tailwindcss').Config} */
export default {
  darkMode: "class",
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "Inter",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Roboto",
          "sans-serif",
        ],
        mono: ["JetBrains Mono", "ui-monospace", "SFMono-Regular", "monospace"],
      },
      colors: {
        accent: {
          DEFAULT: "#2563EB",
          hover: "#1D4ED8",
        },
        status: {
          unlocked: "#10B981",
          locked: "#F59E0B",
          danger: "#EF4444",
        },
      },
    },
  },
  plugins: [],
};
