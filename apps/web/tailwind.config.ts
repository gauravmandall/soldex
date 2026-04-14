import type { Config } from 'tailwindcss'

const config: Config = {
  content: [
    './pages/**/*.{js,ts,jsx,tsx,mdx}',
    './components/**/*.{js,ts,jsx,tsx,mdx}',
    './app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        terminal: {
          bg:        '#0a0b0f',
          secondary: '#0e1018',
          elevated:  '#13151d',
          border:    '#1e2130',
          hover:     '#1a1d28',
          text:      '#e2e4ef',
          muted:     '#8b90a8',
          dim:       '#4b5068',
        },
        brand: {
          green:  '#22c55e',
          red:    '#ef4444',
          blue:   '#3b82f6',
          purple: '#a855f7',
          yellow: '#eab308',
        },
      },
      fontFamily: {
        mono: ['var(--font-mono)', 'JetBrains Mono', 'Fira Code', 'monospace'],
        sans: ['var(--font-sans)', 'Space Grotesk', 'system-ui', 'sans-serif'],
      },
      animation: {
        'flash-green': 'flash-green 0.3s ease',
        'flash-red':   'flash-red 0.3s ease',
        'fade-in':     'fade-in 0.2s ease-out',
      },
      keyframes: {
        'flash-green': {
          '0%, 100%': { color: '#e2e4ef' },
          '50%':       { color: '#22c55e' },
        },
        'flash-red': {
          '0%, 100%': { color: '#e2e4ef' },
          '50%':       { color: '#ef4444' },
        },
        'fade-in': {
          from: { opacity: '0', transform: 'translateY(4px)' },
          to:   { opacity: '1', transform: 'translateY(0)' },
        },
      },
      lineClamp: {
        2: '2',
        3: '3',
      },
    },
  },
  plugins: [],
}

export default config
