/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./app/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./hooks/**/*.{ts,tsx}",
    "./providers/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        mono: ["'IBM Plex Mono'", "monospace"],
        sans: ["'DM Sans'", "system-ui", "sans-serif"],
        display: ["'Space Grotesk'", "sans-serif"],
      },
      colors: {
        bg: {
          base: "#080b0f",
          panel: "#0d1117",
          elevated: "#141c24",
          border: "#1e2d3d",
        },
        accent: {
          green: "#00d4a0",
          red: "#ff4a6b",
          blue: "#3b82f6",
          amber: "#f59e0b",
          purple: "#8b5cf6",
        },
        text: {
          primary: "#e6edf3",
          secondary: "#7d8590",
          muted: "#484f58",
        },
      },
      keyframes: {
        "fade-in": {
          from: { opacity: "0", transform: "translateY(4px)" },
          to: { opacity: "1", transform: "translateY(0)" },
        },
        "flash-green": {
          "0%, 100%": { backgroundColor: "transparent" },
          "50%": { backgroundColor: "rgba(0, 212, 160, 0.15)" },
        },
        "flash-red": {
          "0%, 100%": { backgroundColor: "transparent" },
          "50%": { backgroundColor: "rgba(255, 74, 107, 0.15)" },
        },
        pulse: {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.4" },
        },
      },
      animation: {
        "fade-in": "fade-in 0.2s ease-out",
        "flash-green": "flash-green 0.4s ease",
        "flash-red": "flash-red 0.4s ease",
        pulse: "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite",
      },
    },
  },
  plugins: [],
};
