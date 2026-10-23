import { existsSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { DevupUIWebpackPlugin } from '../plugin'

it('clears the theme and creates the CSS directory when configuration cannot be read', () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-webpack-config-'))
  const config = join(root, 'devup.json')
  const cssDir = join(root, 'dist/css')
  mkdirSync(config)
  wasm.registerTheme({ colors: { default: { brand: '#123456' } } })
  expect(wasm.getDefaultTheme()).toBe('default')
  const registered = spyOn(wasm, 'registerTheme')
  const logged = spyOn(console, 'error').mockReturnValue(undefined)
  try {
    const plugin = new DevupUIWebpackPlugin({
      devupFile: config,
      distDir: join(root, 'dist'),
      cssDir,
      watch: false,
    })

    plugin.writeDataFiles()

    expect(registered).toHaveBeenCalledTimes(1)
    expect(registered).toHaveBeenCalledWith({})
    expect(wasm.getDefaultTheme()).toBeUndefined()
    expect(logged).toHaveBeenCalledTimes(1)
    expect(logged).toHaveBeenCalledWith(expect.any(Error))
    expect(existsSync(cssDir)).toBe(true)
  } finally {
    registered.mockRestore()
    logged.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
})
