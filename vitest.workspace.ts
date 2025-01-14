import { defineWorkspace } from 'vitest/config'

export default defineWorkspace([
  {
    test: {
      name: 'node',
      include: ['packages/*/src/**/__tests__/**/*.test.*'],
      globals: true,
    },
  },
])
