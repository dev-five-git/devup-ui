import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import type { BunPlugin, PluginBuilder } from 'bun'
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
import { register } from '../plugin'

type LoadCallback = Parameters<PluginBuilder['onLoad']>[1]
type ResolveCallback = Parameters<PluginBuilder['onResolve']>[1]
const root = fs.mkdtempSync(join(tmpdir(), 'devup-ui-bun-hooks-'))
const scopedRoot = join(root, '@devup-ui/fixture')
fs.mkdirSync(scopedRoot, { recursive: true })
const realPlugin = Bun.plugin
const loads: {
  options: Parameters<PluginBuilder['onLoad']>[0]
  callback: LoadCallback
}[] = []
const resolves: ResolveCallback[] = []
let exists: ReturnType<typeof spyOn<typeof fs, 'existsSync'>>
let readConfig: ReturnType<typeof spyOn<typeof fsPromises, 'readFile'>>
let write: ReturnType<typeof spyOn<typeof fsPromises, 'writeFile'>>
let writeSync: ReturnType<typeof spyOn<typeof fs, 'writeFileSync'>>
let mkdir: ReturnType<typeof spyOn<typeof fsPromises, 'mkdir'>>

beforeEach(() => {
  loads.length = 0
  resolves.length = 0
  exists = spyOn(fs, 'existsSync').mockReturnValue(false)
  readConfig = spyOn(fsPromises, 'readFile').mockResolvedValue('{}')
  write = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  writeSync = spyOn(fs, 'writeFileSync').mockReturnValue(undefined)
  mkdir = spyOn(fsPromises, 'mkdir').mockResolvedValue(undefined)
  const capturePlugin = Object.assign(
    <T extends BunPlugin>(config: T) =>
      realPlugin({
        ...config,
        setup(builder) {
          builder.onLoad = (options, callback) => {
            loads.push({ options, callback })
            return builder
          }
          builder.onResolve = (_options, callback) => {
            resolves.push(callback)
            return builder
          }
          return config.setup(builder)
        },
      }),
    { clearAll: realPlugin.clearAll },
  )
  bunRegistration.mockImplementation(capturePlugin)
})

afterEach(() => {
  bunRegistration.mockRestore()
  bunRegistration.mockImplementation(realPlugin)
  exists.mockRestore()
  readConfig.mockRestore()
  write.mockRestore()
  writeSync.mockRestore()
  mkdir.mockRestore()
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
