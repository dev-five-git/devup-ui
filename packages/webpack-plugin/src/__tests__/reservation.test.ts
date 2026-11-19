import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, relative, resolve } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import { expect, it, mock, spyOn } from 'bun:test'

import { DevupUIWebpackPlugin } from '../plugin'
import { createWebpackReservation } from '../reservation'

const wasm = utils.loadCompiler('wasm')

it('maps the physical inventory to the exact loader IDs, CSS directory and flags', () => {
  const input = {
    physicalPath: resolve('physical', 'A.js'),
    filename: 'physical/A.js',
    code: 'source',
  }
  const reserve = mock<utils.SharedSlotReservation['reserve']>(
    (discovery, options) => {
      expect(discovery.roots).toEqual([
        resolve('src'),
        resolve('app'),
        resolve('context', 'src'),
        resolve('context', 'app'),
      ])
      // The loader IDs are relative to webpack's context, the project root
      expect(discovery.toId?.(input.physicalPath)).toBe('../physical/A.js')
      expect(discovery.cwd).toBe(resolve('context'))
      expect(discovery.tsconfigPath).toBe(resolve('context', 'tsconfig.json'))
      expect(discovery.include).toEqual(['@slot/ui'])
      expect(discovery.package).toBe('@devup-ui/react')
      expect(options.singleCss).toBe(true)
      expect(options.importMainCssInCode).toBe(false)
      expect(options.importMainCssInCss).toBe(true)
      expect(options.cssDir(input)).toBe(
        `./${relative(dirname(input.physicalPath), resolve('df', 'devup-ui')).replaceAll('\\', '/')}`,
      )
      return true
    },
  )
  const manager = { invalidate: mock(), reserve }
  const create = spyOn(utils, 'createSharedSlotReservation').mockReturnValue(
    manager,
  )
  const setResolver = spyOn(wasm, 'setModuleResolver').mockReturnValue(
    undefined,
  )
  try {
    const plugin = new DevupUIWebpackPlugin({
      engine: 'wasm',
      include: ['@slot/ui'],
      singleCss: true,
    })
    const selected = utils.loadCompiler('wasm')
    const barrier = createWebpackReservation(plugin.options, {}, selected)
    barrier({
      context: resolve('context'),
      entry: () => {
        throw new Error('entry must stay opaque')
      },
    })
    expect(create).toHaveBeenCalledWith(selected)
    expect(manager.invalidate).toHaveBeenCalledTimes(1)
    expect(reserve).toHaveBeenCalledTimes(1)
    expect(setResolver).toHaveBeenCalledTimes(1)
  } finally {
    create.mockRestore()
    setResolver.mockRestore()
  }
})

it("resolves the imports of root-relative ids under webpack's context when started from a parent directory", () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-ui-reservation-root-'))
  const create = spyOn(utils, 'createSharedSlotReservation').mockReturnValue({
    invalidate: mock(),
    reserve: mock().mockReturnValue(true),
  })
  const setResolver = spyOn(wasm, 'setModuleResolver').mockReturnValue(
    undefined,
  )
  try {
    mkdirSync(join(root, 'src'), { recursive: true })
    writeFileSync(
      join(root, 'tsconfig.json'),
      JSON.stringify({
        compilerOptions: { paths: { '@root-fixture/*': ['./src/*'] } },
      }),
    )
    writeFileSync(join(root, 'src', 'tokens.ts'), 'export const a = 1')
    const plugin = new DevupUIWebpackPlugin({ engine: 'wasm' })
    createWebpackReservation(
      plugin.options,
      {},
      utils.loadCompiler('wasm'),
    )({ context: root, entry: {} })
    const resolveModule = setResolver.mock.calls[0]?.[0]
    assert(typeof resolveModule === 'function')
    for (const specifier of ['./tokens', '@root-fixture/tokens'])
      expect(resolveModule(specifier, 'src/App.tsx')).toEqual({
        path: 'src/tokens.ts',
        code: 'export const a = 1',
      })
  } finally {
    create.mockRestore()
    setResolver.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
})
