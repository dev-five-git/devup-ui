import { DevupUI } from '@devup-ui/vite-plugin'
import mdx from '@mdx-js/rollup'
import vinext from 'vinext'
import { defineConfig } from 'vite'

export default defineConfig({
  define: {
    'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify('light'),
  },
  plugins: [
    DevupUI({ singleCss: process.env.DEVUP_SINGLE_CSS === '1' }),
    mdx({ providerImportSource: '@/mdx-components' }),
    vinext(),
  ],
})
