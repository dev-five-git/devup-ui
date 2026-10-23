import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, relative, resolve } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, mock, spyOn } from 'bun:test'

import { DevupUIWebpackPlugin } from '../plugin'
import { createWebpackReservation } from '../reservation'

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
      expect(discovery.toId?.(input.physicalPath)).toBe('../physical/A.js')
      expect(discovery.include).toEqual(['@slot/ui'])
      expect(discovery.package).toBe('@devup-ui/react')
      expect(discovery.entries).toEqual([])
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
      include: ['@slot/ui'],
      singleCss: true,
    })
    const barrier = createWebpackReservation(plugin.options, {})
    barrier({
      context: resolve('context'),
      entry: () => {
        throw new Error('entry must stay opaque')
      },
    })
    expect(create).toHaveBeenCalledWith({
      reserveClassSlots: wasm.reserveClassSlots,
    })
    expect(manager.invalidate).toHaveBeenCalledTimes(1)
    expect(reserve).toHaveBeenCalledTimes(1)
    expect(setResolver).not.toHaveBeenCalled()
  } finally {
    create.mockRestore()
    setResolver.mockRestore()
  }
})

it('forwards only normalized physical entry requests with their original context', () => {
  // Given: Webpack loader syntax, virtual entries, bare packages and drive paths.
  const context = resolve('entry-context')
  const resources = [
    './outside/main.ts',
    '/outside/main.ts',
    'entry-package',
    '@slot/entry',
    'C:\\outside\\main.ts',
    'C:/outside/main.ts',
    './outside/\0.ts',
  ]
  const reserve = mock<utils.SharedSlotReservation['reserve']>(() => true)
  const create = spyOn(utils, 'createSharedSlotReservation').mockReturnValue({
    invalidate: mock(),
    reserve,
  })
  try {
    const plugin = new DevupUIWebpackPlugin()
    const barrier = createWebpackReservation(plugin.options, {})
    // When: discovery crosses the adapter boundary, not the canonical graph.
    barrier({
      context,
      entry: {
        empty: {},
        main: {
          import: [
            `next-app-loader?page=${'segment/'.repeat(5000)}!`,
            '',
            'loader?pitch!?query',
            'loader!\0virtual',
            'virtual:entry',
            'loader!data:text/javascript,entry',
            'loader!file:///outside/main.ts',
            'loader!C:relative.ts',
            ...resources.map(
              (resource) => `first?query!second?query!${resource}?resource`,
            ),
          ],
        },
      },
    })
    // Then: neither loader metadata nor virtual IDs enter physical resolution.
    expect(reserve.mock.calls[0]?.[0].entries).toEqual(
      resources.map((request) => ({ request, context })),
    )
    expect(reserve.mock.calls[0]?.[0].cwd).toBe(context)
  } finally {
    create.mockRestore()
  }
})

it.each(['outside', 'package'] as const)(
  'reserves the real %s entry closure without enumerating unrelated siblings',
  async (kind) => {
    // Given: a real entry outside conventional roots with a relative supplier.
    const root = await mkdtemp(join(tmpdir(), 'webpack-entry-'))
    const directory =
      kind === 'package'
        ? join(root, 'node_modules/entry-package')
        : join(root, 'external')
    const reserve = spyOn(wasm, 'reserveClassSlots').mockReturnValue(undefined)
    try {
      await mkdir(directory, { recursive: true })
      await writeFile(
        join(directory, 'package.json'),
        JSON.stringify({
          name: 'entry-package',
          exports: './entry.js',
        }),
      )
      await writeFile(
        join(directory, 'entry.js'),
        "import {css} from '@devup-ui/react'; import {color} from './value.js'; export const style=css({color})",
      )
      await writeFile(join(directory, 'value.js'), "export const color='red'")
      await writeFile(
        join(directory, 'unused.js'),
        "import {css} from '@devup-ui/react'; export const style=css({color:'blue'})",
      )
      const plugin = new DevupUIWebpackPlugin()
      const barrier = createWebpackReservation(plugin.options, {})
      const request =
        kind === 'package' ? 'entry-package' : './external/entry.js'
      // When: the real shared collector resolves the normalized entry request.
      barrier({
        context: root,
        entry: { main: { import: [`loader!${request}?query`] } },
      })
      // Then: the engine receives exactly the entry and its supplier, not siblings.
      expect(reserve.mock.calls[0]?.[0].map(([filename]) => filename)).toEqual(
        ['entry.js', 'value.js'].map((file) =>
          relative(root, join(directory, file)).replaceAll('\\', '/'),
        ),
      )
    } finally {
      reserve.mockRestore()
      await rm(root, { recursive: true, force: true })
    }
  },
)
