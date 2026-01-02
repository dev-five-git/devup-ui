import { describe, expect, it, mock } from 'bun:test'

mock.module('@devup-ui/webpack-plugin', () => ({
  DevupUIWebpackPlugin: mock(),
}))

mock.module('@devup-ui/wasm', () => ({
  registerTheme: mock(),
  getThemeInterface: mock(() => ''),
  getDefaultTheme: mock(),
  getCss: mock(() => ''),
  setPrefix: mock(),
  exportSheet: mock(() => '{}'),
  exportClassMap: mock(() => '{}'),
  exportFileMap: mock(() => '{}'),
}))

describe('export', () => {
  it('should export DevupUI', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      DevupUI: expect.any(Function),
    })
  })
})
