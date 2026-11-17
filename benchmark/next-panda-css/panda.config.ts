import { defineConfig } from '@pandacss/dev'

export default defineConfig({
  presets: ['@pandacss/preset-base', '@pandacss/preset-panda'],

  // No css reset, like the other benchmark fixtures
  preflight: false,

  // Where to look for your css declarations
  include: ['./src/**/*.{js,jsx,ts,tsx}', './pages/**/*.{js,jsx,ts,tsx}'],

  // Files to exclude
  exclude: [],

  // Match the 480px second breakpoint the other fixtures use
  theme: {
    extend: {
      breakpoints: {
        sm: '30rem',
      },
    },
  },
  jsxFramework: 'react',

  // The output directory for your css system
  outdir: 'styled-system',
})
