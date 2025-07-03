import { DevupUI } from '@devup-ui/vite-plugin'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      include: ['packages/*/src/**'],
      exclude: ['packages/*/src/types', 'packages/*/src/**/__tests__'],
    },
    projects: [
      {
        test: {
          name: 'node',
          include: ['packages/*/src/**/__tests__/**/*.test.{ts,tsx}'],
          exclude: ['packages/*/src/**/__tests__/**/*.browser.test.{ts,tsx}'],
          globals: true,
        },
      },
      {
        test: {
          include: ['packages/*/src/**/__tests__/**/*.browser.test.{ts,tsx}'],
          name: 'happy-dom',
          environment: 'happy-dom',
          globals: true,
          css: true,
          setupFiles: ['@testing-library/jest-dom/vitest'],
        },
        plugins: [DevupUI()],
      },
    ],
  },
})
