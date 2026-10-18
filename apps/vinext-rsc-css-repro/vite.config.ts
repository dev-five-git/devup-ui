import { DevupUI } from '@devup-ui/vite-plugin'
import vinext from 'vinext'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    DevupUI({ singleCss: process.env.DEVUP_SINGLE_CSS === '1' }),
    vinext(),
  ],
})
