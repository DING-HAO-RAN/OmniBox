/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        // Windows 11 Fluent 拟态调色板
        fluent: {
          bg: "#0d1117",
          surface: "#161b22",
          card: "rgba(255, 255, 255, 0.04)",
          border: "rgba(255, 255, 255, 0.1)",
          accent: "#0078d4",
          accentHover: "#1084d9",
        },
      },
    },
  },
  plugins: [],
};
