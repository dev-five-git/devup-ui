import { describe, expect, it, mock } from 'bun:test'

mock.module('@devup-ui/react', () => ({
  globalCss: mock(),
}))

// Dynamic import AFTER mock.module for proper mocking
const { resetCss } = await import('../index')

describe('reset-css', () => {
  it('should be defined', () => {
    expect(resetCss).toBeInstanceOf(Function)
    expect(resetCss()).toBeUndefined()
  })
})
