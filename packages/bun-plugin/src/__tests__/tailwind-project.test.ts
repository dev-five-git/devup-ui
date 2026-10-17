import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { runBuildOperation } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, mock, spyOn } from 'bun:test'

import * as watchers from '../tailwind-watch'
import { fixture } from './callback-fixture.test'

it('reloads Tailwind imports using the Bun project root and configured output paths', async () => {
  // Given
  const f = fixture()
  const css = f.write('styles/main.css', '@import "./tokens.css";')
  const tokens = f.write(
    'styles/tokens.css',
    '@theme { --color-brand: #123456; }',
  )
  f.write('config/theme.json', '{"tailwind":{"css":"styles/main.css"}}')
  let reload: (() => Promise<string[]>) | undefined
  const pending = Promise.withResolvers<void>()
  const close = mock(() => pending.promise)
  const watchFiles = Object.assign(
    mock((_files: string[]) => {}),
    { close },
  )
  const watcher = spyOn(watchers, 'createTailwindWatcher').mockImplementation(
    (callback) => {
      reload = callback
      return watchFiles
    },
  )
  const register = spyOn(wasm, 'registerTheme')
  try {
    await f.setup({ devupFile: 'config/theme.json', distDir: 'generated' })
    expect(watchFiles).toHaveBeenCalledWith([css, tokens])
    f.write('styles/tokens.css', '@theme { --color-brand: #abcdef; }')
    f.write(
      'config/theme.json',
      '{"tailwind":{"css":"styles/main.css"},"theme":{"colors":{"default":{"refreshed":"#135790"}}}}',
    )
    // When
    if (!reload)
      throw new Error('setup did not install the Tailwind reload callback')
    const files = await reload()
    // Then
    expect(files).toEqual([css, tokens])
    expect(register).toHaveBeenLastCalledWith({
      colors: { default: { refreshed: '#135790' } },
      tailwindCss: '@theme { --color-brand: #abcdef; }',
    })
    expect(
      readFileSync(join(f.root, 'generated/theme.d.ts'), 'utf-8'),
    ).toContain('refreshed')
    const cleanup = f.cleanup()
    let finished = false
    void cleanup.then(() => {
      finished = true
    })
    await Promise.resolve()
    expect(close).toHaveBeenCalledTimes(1)
    expect(finished).toBe(false)
    expect(() =>
      runBuildOperation({ integration: 'probe', root: f.root }, () => {}),
    ).toThrow('cannot activate')
    pending.resolve()
    await cleanup
  } finally {
    pending.resolve()
    watcher.mockRestore()
    register.mockRestore()
    await f.cleanup()
  }
})
