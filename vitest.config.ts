import { DevupUI } from '@devup-ui/vite-plugin'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      include: ['packages/*/src/**'],
      exclude: [
        'packages/*/src/types',
        'packages/*/src/**/__tests__',
        '**/*.stories.{ts,tsx}',
        '*.md',
        '*.mdx',
      ],
      cleanOnRerun: true,
      reporter: ['text', 'json', 'html'],
      thresholds: {
        100: true,
      },
    },
    projects: [
      {
        test: {
          name: 'node',
          include: ['packages/*/src/**/__tests__/**/*.test.{ts,tsx}'],
          exclude: ['packages/*/src/**/__tests__/**/*.browser.test.{ts,tsx}'],
          globals: true,
          setupFiles: ['./vitest.setup.ts'],
          environment: 'node',
        },
      },
      {
        test: {
          name: 'happy-dom',
          include: ['packages/*/src/**/__tests__/**/*.browser.test.{ts,tsx}'],
          environment: 'happy-dom',
          globals: true,
          css: true,
          setupFiles: ['./vitest.setup.ts', '@testing-library/jest-dom/vitest'],
        },
        plugins: [
          DevupUI({
            debug: true,
            singleCss: true,
          }),
        ],
      },
    ],
    cache: false,
  },
})
