import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import * as nodePath from 'node:path'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import {
  afterAll,
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  spyOn,
} from 'bun:test'

import { bunRegistration } from '../../../../bun.setup'
import { DevupUI, register } from '../plugin'
import { captureBunHooks } from './captured-hooks.test-support'

const root = fs.mkdtempSync(join(tmpdir(), 'devup-ui-bun-hooks-'))
const scopedRoot = join(root, '@devup-ui/fixture')
fs.mkdirSync(scopedRoot, { recursive: true })
const realPlugin = Bun.plugin
const realExists = fs.existsSync
const realResolve = nodePath.resolve
let hooks: ReturnType<typeof captureBunHooks>
let loads: typeof hooks.loads
let resolves: typeof hooks.resolves
let exists: typeof hooks.exists
let readConfig: typeof hooks.readConfig
let write: typeof hooks.write
let writeSync: typeof hooks.writeSync
let mkdir: typeof hooks.mkdir

beforeEach(() => {
  hooks = captureBunHooks()
  ;({ loads, resolves, exists, readConfig, write, writeSync, mkdir } = hooks)
})

afterEach(async () => {
  await hooks.cleanup()
})
afterAll(() => fs.rmSync(root, { recursive: true, force: true }))

async function load(path: string, namespace = 'file') {
  const hook = loads.find(
    ({ options }) =>
      (options.namespace ?? 'file') === namespace && options.filter.test(path),
  )
  if (!hook) throw new Error(`No load hook for ${path}`)
  return hook.callback({
    path,
    namespace,
    loader: 'file',
    defer: async () => {},
  })
}

describe('registered Bun hooks', () => {
  it('seeds readable names from physical inputs before runtime source loads', async () => {
    const file = join(root, 'numbered.tsx')
    await Bun.write(
      file,
      "import {Box} from '@devup-ui/react'; export const view=<Box color='purple'/>",
    )
    exists.mockImplementation(realExists)
    const resolve = spyOn(nodePath, 'resolve').mockImplementation((...paths) =>
      paths.length === 1 && paths[0] === 'src' ? root : realResolve(...paths),
    )
    try {
      await register()
      expect(Object.keys(JSON.parse(wasm.exportFileMap()))).toContain(file)
      expect(wasm.isDebug()).toBe(true)
    } finally {
      resolve.mockRestore()
    }
  })

  it('retains native registration and clearAll identity', () => {
    expect(Bun.plugin).toBe(realPlugin)
    expect(bunRegistration.clearAll).toBe(realPlugin.clearAll)
  })

  it('reserves short runtime names during awaited setup', async () => {
    await register({ debug: false })
    expect(wasm.isDebug()).toBe(false)
    expect(loads.length).toBe(2)
  })

  it('leaves invalid JavaScript to the native loader after scanning fails', async () => {
    const path = join(root, 'malformed.ts')
    const source = 'import {'
    await Bun.write(path, source)
    await register()
    expect(await load(path)).toEqual({ contents: source, loader: 'ts' })
    expect(writeSync).not.toHaveBeenCalled()
  })

  it('awaits real build setup and deferred CSS through two native build lifecycles', async () => {
    const entry = join(root, 'bundle.tsx')
    await Bun.write(
      entry,
      "import {Box} from '@devup-ui/react'; export const view=<Box color='tomato'/>;",
    )
    const outputs: string[][] = []
    for (let round = 0; round < 2; round += 1) {
      const result = await Bun.build({
        entrypoints: [entry],
        target: 'bun',
        external: ['react/jsx-dev-runtime', 'react/jsx-runtime'],
        outdir: join(root, 'out'),
        plugins: [DevupUI()],
      })
      expect(result.success).toBe(true)
      expect(result.logs).toEqual([])
      const css = await Promise.all(
        result.outputs
          .filter((output) => output.path.endsWith('.css'))
          .map((output) => output.text()),
      )
      expect(css.join('')).toMatch(/color:\s*tomato/)
      outputs.push(css)
    }
    expect(outputs[1]).toEqual(outputs[0])
    expect(writeSync).not.toHaveBeenCalled()
  })

  it('creates cold directories before writing configuration and CSS', async () => {
    await register()
    expect(mkdir).toHaveBeenCalledWith('df', { recursive: true })
    expect(mkdir).toHaveBeenCalledWith(expect.stringContaining('devup-ui'), {
      recursive: true,
    })
    expect(write).toHaveBeenCalledWith(
      join('df', 'theme.d.ts'),
      expect.any(String),
      'utf-8',
    )
    expect(wasm.isDebug()).toBe(true)
  })

  it('loads the configured theme through the public registration API', async () => {
    exists.mockReturnValue(true)
    readConfig.mockResolvedValue(
      '{"theme":{"colors":{"default":{"primary":"#123456"}}}}',
    )
    await register()
    expect(wasm.getCss(null, false)).toContain('#123456')
    expect(mkdir).not.toHaveBeenCalled()
  })

  it('registers an empty theme when reading configuration fails', async () => {
    exists.mockReturnValue(true)
    readConfig.mockRejectedValue(new Error('unreadable fixture config'))
    await register()
    expect(wasm.getDefaultTheme()).toBeUndefined()
  })

  it('resolves generated stylesheets and loads their runtime placeholder', async () => {
    await register()
    const resolve = resolves[0]
    if (!resolve) throw new Error('No CSS resolver registered')
    const result = await resolve({
      path: './df/devup-ui/devup-ui.css',
      importer: join(process.cwd(), 'entry.tsx'),
      namespace: 'file',
      kind: 'import-statement',
      resolveDir: process.cwd(),
    })
    if (!result?.namespace || !result.path)
      throw new Error('Expected virtual CSS resolution')
    expect(await load(result.path, result.namespace)).toEqual({
      contents: '',
      loader: 'js',
    })
  })

  for (const [extension, loader] of [
    ['tsx', 'tsx'],
    ['ts', 'ts'],
    ['jsx', 'jsx'],
    ['mjs', 'js'],
    ['js', 'js'],
  ] as const) {
    it(`compiles a ${extension} source through the captured onLoad callback`, async () => {
      const source =
        extension === 'tsx' || extension === 'jsx'
          ? "import { Box } from '@devup-ui/react'; export const view = <Box bg='red' />"
          : "import { css } from '@devup-ui/react'; export const className = css({ color: 'red' })"
      const path = join(
        extension === 'js' ? scopedRoot : root,
        `source.${extension}`,
      )
      await Bun.write(path, source)
      await register()
      const result = await load(path)
      if (!result || !('contents' in result))
        throw new Error('Expected transformed source contents')
      expect(result?.loader).toBe(loader)
      expect(result.contents).not.toContain(
        extension === 'tsx' || extension === 'jsx' ? '<Box' : 'css(',
      )
      expect(writeSync).toHaveBeenCalledWith(
        expect.stringContaining('devup-ui.css'),
        expect.stringContaining('red'),
        'utf-8',
      )
    })
  }

  it('returns source without style imports unchanged', async () => {
    const path = join(root, 'plain.ts')
    await Bun.write(path, 'export const value = 42')
    await register()
    expect(await load(path)).toEqual({
      contents: 'export const value = 42',
      loader: 'ts',
    })
    expect(writeSync).not.toHaveBeenCalled()
  })

  it('propagates source read errors instead of returning partial output', async () => {
    await register()
    await expect(load(join(root, 'missing.ts'))).rejects.toThrow()
  })

  it('propagates build errors through the actual callback', async () => {
    const path = join(root, 'invalid.ts')
    await Bun.write(
      path,
      "import { css } from '@devup-ui/react'; export const styles = css({ color: window.name })",
    )
    await register()
    await expect(load(path)).rejects.toThrow(/invalid\.ts:\d+:\d+:/)
  })
})
