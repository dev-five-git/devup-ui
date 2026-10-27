import { existsSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { ConfigLoadError } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { DevupUIWebpackPlugin } from '../plugin'

it('rejects unreadable configuration without publishing or clearing the prior theme', () => {
  // Given
  const root = mkdtempSync(join(tmpdir(), 'devup-webpack-config-'))
  const config = join(root, 'devup.json')
  const cssDir = join(root, 'dist/css')
  mkdirSync(config)
  wasm.registerTheme({ colors: { default: { brand: '#123456' } } })
  expect(wasm.getDefaultTheme()).toBe('default')
  const priorCss = wasm.getCss(null, false)
  const registered = spyOn(wasm, 'registerTheme')
  const logged = spyOn(console, 'error').mockReturnValue(undefined)
  try {
    const plugin = new DevupUIWebpackPlugin({
      devupFile: config,
      distDir: join(root, 'dist'),
      cssDir,
      watch: false,
    })

    // When
    let failure: unknown
    try {
      plugin.writeDataFiles()
    } catch (error) {
      if (!(error instanceof ConfigLoadError)) throw error
      failure = error
    }
    // Then
    expect(failure).toBeInstanceOf(ConfigLoadError)
    if (!(failure instanceof ConfigLoadError))
      throw new TypeError('Expected propagated config load error')
    expect(failure.file).toBe(config)
    expect(failure.message).toContain(`${config}:1:1:`)
    expect(failure.cause).toBeInstanceOf(Error)
    expect(failure.cause).toMatchObject({ code: 'EISDIR' })
    expect(registered).not.toHaveBeenCalled()
    expect(wasm.getDefaultTheme()).toBe('default')
    expect(wasm.getCss(null, false)).toBe(priorCss)
    expect(logged).not.toHaveBeenCalled()
    expect(existsSync(cssDir)).toBe(false)
    expect(existsSync(join(root, 'dist/theme.d.ts'))).toBe(false)
  } finally {
    registered.mockRestore()
    logged.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
})
