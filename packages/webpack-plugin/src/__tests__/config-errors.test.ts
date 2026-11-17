import { existsSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { loadCompiler } from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'

import { DevupUIWebpackPlugin } from '../plugin'

const wasm = loadCompiler('wasm')

function withUnreadableConfig(
  test: (paths: { config: string; root: string; cssDir: string }) => void,
) {
  const root = mkdtempSync(join(tmpdir(), 'devup-webpack-config-'))
  const config = join(root, 'devup.json')
  mkdirSync(config)
  try {
    test({ config, root, cssDir: join(root, 'dist/css') })
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

it('fails a one-shot build when configuration cannot be read', () => {
  withUnreadableConfig(({ config, root, cssDir }) => {
    const plugin = new DevupUIWebpackPlugin({
      engine: 'wasm',
      devupFile: config,
      distDir: join(root, 'dist'),
      cssDir,
      watch: false,
    })

    expect(() => plugin.writeDataFiles()).toThrow()
    expect(plugin.themeError).toBeUndefined()
  })
})

it('clears the theme and creates the CSS directory when configuration cannot be read in watch mode', () => {
  withUnreadableConfig(({ config, root, cssDir }) => {
    wasm.registerTheme({ colors: { default: { brand: '#123456' } } })
    expect(wasm.getDefaultTheme()).toBe('default')
    const registered = spyOn(wasm, 'registerTheme')
    const logged = spyOn(console, 'error').mockReturnValue(undefined)
    try {
      const plugin = new DevupUIWebpackPlugin({
        engine: 'wasm',
        devupFile: config,
        distDir: join(root, 'dist'),
        cssDir,
        watch: true,
      })

      plugin.writeDataFiles()

      expect(registered).toHaveBeenCalledTimes(1)
      expect(registered).toHaveBeenCalledWith({})
      expect(wasm.getDefaultTheme()).toBeUndefined()
      expect(logged).toHaveBeenCalledTimes(1)
      expect(logged).toHaveBeenCalledWith(expect.any(Error))
      expect(plugin.themeError).toBeInstanceOf(Error)
      expect(existsSync(cssDir)).toBe(true)
    } finally {
      registered.mockRestore()
      logged.mockRestore()
    }
  })
})
