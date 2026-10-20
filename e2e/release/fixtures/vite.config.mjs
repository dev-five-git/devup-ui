import { DevupUI } from '@devup-ui/vite-plugin'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    react(),
    DevupUI({ singleCss: process.env.RELEASE_SINGLE_CSS === '1' }),
  ],
})
