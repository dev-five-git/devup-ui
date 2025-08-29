import { DevupUI } from '@devup-ui/vite-plugin'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    DevupUI({
      include: ['vite-lib-example'],
      // singleCss: true,
    }),
  ],
})
