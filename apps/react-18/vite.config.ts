import { DevupUI } from '@devup-ui/vite-plugin'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  // The workspace packages keep React 19 for their own tests; a consumer
  // install resolves their React peer to the app's single copy, as this does
  resolve: { dedupe: ['react', 'react-dom'] },
  plugins: [
    react(),
    DevupUI({
      include: ['@devup-ui/components'],
      singleCss: true,
    }),
  ],
})
