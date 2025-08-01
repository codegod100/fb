/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.rs",
    "./dist/index.html"
  ],
  theme: {
    extend: {
      colors: {
        // Catppuccin Mocha color palette
        'ctp': {
          'base': '#1e1e2e',
          'mantle': '#181825',
          'crust': '#11111b',
          'surface0': '#313244',
          'surface1': '#45475a',
          'surface2': '#585b70',
          'overlay0': '#6c7086',
          'overlay1': '#7f849c',
          'overlay2': '#9399b2',
          'subtext0': '#a6adc8',
          'subtext1': '#bac2de',
          'text': '#cdd6f4',
          'lavender': '#b4befe',
          'blue': '#89b4fa',
          'sapphire': '#74c7ec',
          'sky': '#89dceb',
          'teal': '#94e2d5',
          'green': '#a6e3a1',
          'yellow': '#f9e2af',
          'peach': '#fab387',
          'maroon': '#eba0ac',
          'red': '#f38ba8',
          'mauve': '#cba6f7',
          'pink': '#f5c2e7',
          'flamingo': '#f2cdcd',
          'rosewater': '#f5e0dc',
        }
      },
      animation: {
        'fade-in': 'fadeIn 0.3s ease-in-out'
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' }
        }
      }
    },
  },
  plugins: [],
}