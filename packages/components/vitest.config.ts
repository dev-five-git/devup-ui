import { DevupUI } from '@devup-ui/vite-plugin'
import { defineProject } from 'vitest/config'

export default defineProject({
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./setupTests.ts'],
  },
  plugins: [DevupUI()],
})
