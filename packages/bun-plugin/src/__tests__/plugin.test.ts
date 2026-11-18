import * as fs from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import * as nodePath from 'node:path'
import { join } from 'node:path'

import * as pluginUtils from '@devup-ui/plugin-utils'
import { ShorthandConfigError } from '@devup-ui/plugin-utils'
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
import { bunConditions, DevupUI, register } from '../plugin'
import { captureBunHooks } from './captured-hooks.test-support'

const selectCompiler = pluginUtils.loadCompiler
const wasm = selectCompiler('wasm')

describe('shorthand creation boundary', () => {
  it.each([undefined, 'native', 'wasm'] as const)(
    'selects engine=%s once per factory',
    (engine) => {
      const selection = spyOn(pluginUtils, 'loadCompiler').mockReturnValue(wasm)
      try {
        DevupUI(engine === undefined ? undefined : { engine })
        expect(selection.mock.calls).toEqual([[engine]])
      } finally {
        selection.mockRestore()
        selection.mockImplementation(selectCompiler)
      }
    },
  )
  it('registers canonical targets when the plugin is registered', async () => {
    const registration = spyOn(wasm, 'registerShorthands').mockReturnValue(
      undefined,
    )
    try {
      await register({
        engine: 'wasm',
        shorthands: { bgx: ['backgroundColor', 'py', '--Gap'] },
      })
      expect(registration.mock.calls).toEqual([
        [
          {
            bgx: ['background-color', 'padding-top', 'padding-bottom', '--Gap'],
          },
        ],
      ])
    } finally {
      registration.mockRestore()
    }
  })

  it('rejects a typo before registration when runtime options bypass types', () => {
    const selection = spyOn(pluginUtils, 'loadCompiler')
    const registration = spyOn(wasm, 'registerShorthands').mockReturnValue(
      undefined,
    )
    try {
      expect(() =>
        Reflect.apply(register, undefined, [
          { shorthands: { bgx: ['backgroundColour'] } },
        ]),
      ).toThrow(ShorthandConfigError)
      expect(registration).not.toHaveBeenCalled()
      expect(selection).not.toHaveBeenCalled()
    } finally {
      registration.mockRestore()
      selection.mockRestore()
      selection.mockImplementation(selectCompiler)
    }
  })

  it('reads StyleX under the styleResolution it is given when registered', async () => {
    const resolution = spyOn(wasm, 'setStylexStyleResolution')
    try {
      await register({
        engine: 'wasm',
        stylex: { styleResolution: 'application-order' },
      })
      expect(resolution.mock.calls).toEqual([['application-order']])
    } finally {
      resolution.mockRestore()
      wasm.setStylexStyleResolution(null)
    }
  })

  it('rejects an unknown styleResolution before selecting a compiler', () => {
    const selection = spyOn(pluginUtils, 'loadCompiler')
    try {
      expect(() =>
        Reflect.apply(register, undefined, [
          { stylex: { styleResolution: 'last-wins' } },
        ]),
      ).toThrow(pluginUtils.StylexConfigError)
      expect(selection).not.toHaveBeenCalled()
    } finally {
      selection.mockRestore()
      selection.mockImplementation(selectCompiler)
    }
  })
})

const root = fs.mkdtempSync(join(tmpdir(), 'devup-ui-bun-hooks-'))
const scopedRoot = join(root, '@devup-ui/fixture')
fs.mkdirSync(scopedRoot, { recursive: true })
// The runtime stub a `require()` the extractor keeps in CommonJS loads
const reactStub = join(root, 'node_modules/@devup-ui/react')
fs.mkdirSync(reactStub, { recursive: true })
fs.writeFileSync(join(reactStub, 'package.json'), '{"main":"index.js"}')
fs.writeFileSync(join(reactStub, 'index.js'), 'exports.css = () => {}')
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

describe('bunConditions', () => {
  it('resolves with the conditions of the target and the configured ones', () => {
    expect(bunConditions(undefined)).toEqual([
      'import',
      'module',
      'bun',
      'node',
    ])
    expect(bunConditions({})).toEqual(['import', 'module', 'browser'])
    expect(bunConditions({ target: 'node', conditions: 'worker' })).toEqual([
      'import',
      'module',
      'node',
      'worker',
    ])
    expect(bunConditions({ target: 'bun', conditions: ['a', 'b'] })).toEqual([
      'import',
      'module',
      'bun',
      'node',
      'a',
      'b',
    ])
  })
})

describe('registered Bun hooks', () => {
  it('compiles default runtime input into native state without mutating WASM', async () => {
    const native = pluginUtils.loadCompiler()
    native.resetStateForTesting()
    const file = join(root, 'native-default.ts')
    await Bun.write(
      file,
      "import {css} from '@devup-ui/react'; export const cls=css({color:'teal'});",
    )
    const before = {
      native: native.getCss(null, false),
      wasm: wasm.getCss(null, false),
    }
    try {
      await register({ debug: false })
      const result = await load(file)
      const after = {
        native: native.getCss(null, false),
        wasm: wasm.getCss(null, false),
      }
      console.info(
        JSON.stringify({ observed: 'native-default', before, after, result }),
      )
      expect(before.native).not.toContain('color:teal')
      expect(result && 'contents' in result && result.contents).not.toContain(
        'css(',
      )
      expect(after.native).toContain('color:teal')
      expect(after.wasm).toBe(before.wasm)
    } finally {
      native.resetStateForTesting()
    }
  })

  it('keeps retained source callbacks on their factory compiler', async () => {
    const native = pluginUtils.loadCompiler()
    native.resetStateForTesting()
    const file = join(root, 'retained-native.ts')
    await Bun.write(
      file,
      "import {css} from '@devup-ui/react'; export const cls=css({color:'teal'});",
    )
    try {
      await register({ engine: 'native', debug: false })
      const retained = loads.splice(0)
      await register({ engine: 'wasm', debug: false })
      const before = {
        native: native.getCss(null, false),
        wasm: wasm.getCss(null, false),
      }
      const source = retained.find(({ options }) => !options.namespace)
      if (!source) throw new Error('Missing retained native source callback')
      const result = await source.callback({
        path: file,
        namespace: 'file',
        loader: 'file',
        defer: async () => {},
      })
      const after = {
        native: native.getCss(null, false),
        wasm: wasm.getCss(null, false),
      }
      console.info(
        JSON.stringify({ observed: 'retained-native', before, after, result }),
      )
      expect(before.native).not.toContain('color:teal')
      expect(result && 'contents' in result && result.contents).not.toContain(
        'css(',
      )
      expect(after.native).toContain('color:teal')
      expect(after.wasm).toBe(before.wasm)
    } finally {
      native.resetStateForTesting()
    }
  })

  it('seeds readable names from physical inputs before runtime source loads', async () => {
    const file = join(root, 'numbered.tsx')
    await Bun.write(
      file,
      "import {Box} from '@devup-ui/react'; export const view=<Box color='purple'/>",
    )
    exists.mockImplementation(realExists)
    const resolve = spyOn(nodePath, 'resolve').mockImplementation((...paths) =>
      paths.length === 2 && paths[0] === process.cwd() && paths[1] === 'src'
        ? root
        : realResolve(...paths),
    )
    try {
      await register({ engine: 'wasm' })
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
    await register({ engine: 'wasm', debug: false })
    expect(wasm.isDebug()).toBe(false)
    expect(loads.length).toBe(2)
  })

  it('turns the license banner off only when asked to', async () => {
    try {
      await register({ engine: 'wasm', licenseBanner: false })
      expect(wasm.isLicenseBanner()).toBe(false)
      await register({ engine: 'wasm', licenseBanner: true })
      expect(wasm.isLicenseBanner()).toBe(true)
      wasm.setLicenseBanner(false)
      await register({ engine: 'wasm' })
      expect(wasm.isLicenseBanner()).toBe(true)
    } finally {
      wasm.setLicenseBanner(true)
    }
  })

  it('warns when runtime file numbering fails, and stays quiet without src', async () => {
    const collect = spyOn(pluginUtils, 'collectNumberedFiles')
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    try {
      collect.mockImplementation(() => {
        throw new Error('scan boom')
      })
      await register({ engine: 'wasm' })
      expect(warn).toHaveBeenCalledWith(
        '[devup-ui] Bun file numbering failed: scan boom. Stylesheet file numbers follow the order Bun loads files in, so they can differ between runs of the same source.',
      )

      warn.mockClear()
      collect.mockImplementation(() => {
        throw Object.assign(new Error('ENOENT'), {
          code: 'ENOENT',
          path: nodePath.resolve('src'),
        })
      })
      await register({ engine: 'wasm' })
      expect(warn).not.toHaveBeenCalled()
    } finally {
      warn.mockRestore()
      collect.mockRestore()
    }
  })

  it('leaves invalid JavaScript to the native loader after scanning fails', async () => {
    const path = join(root, 'malformed.ts')
    const source = 'import {'
    await Bun.write(path, source)
    await register({ engine: 'wasm' })
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
        plugins: [DevupUI({ engine: 'wasm' })],
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

  it('bundles the project .js and .cjs modules and leaves packages to Bun', async () => {
    const dir = join(root, 'local-scripts')
    await Bun.write(
      join(dir, 'entry.ts'),
      "import { view } from './view.js'; import { lib } from 'lib'; export { view, lib }",
    )
    await Bun.write(
      join(dir, 'view.js'),
      "import { css } from '@devup-ui/react'; export const view = css({ color: 'orange' })",
    )
    await Bun.write(
      join(dir, 'node_modules/lib/package.json'),
      JSON.stringify({ name: 'lib', main: 'index.js' }),
    )
    await Bun.write(
      join(dir, 'node_modules/lib/index.js'),
      'export const lib = 1',
    )
    const result = await Bun.build({
      entrypoints: [join(dir, 'entry.ts')],
      target: 'bun',
      outdir: join(dir, 'out'),
      plugins: [DevupUI({ engine: 'wasm' })],
    })
    expect(result.success).toBe(true)
    const outputs = await Promise.all(
      result.outputs.map((output) => output.text()),
    )
    expect(outputs.join('')).toMatch(/color:\s*orange/)
    expect(outputs.join('')).not.toContain('css(')
  })

  it('keeps the build-time dependencies of a source in its module graph', async () => {
    await Bun.write(join(root, 'dep-tokens.ts'), "export const color = 'red'")
    const path = join(root, 'dep-source.ts')
    await Bun.write(
      path,
      "import { css } from '@devup-ui/react'; import { color } from './dep-tokens'; export const c = css({ color })",
    )
    await register({ engine: 'wasm' })
    const result = await load(path)
    if (!result || !('contents' in result))
      throw new Error('Expected transformed source contents')
    expect(result.contents).toEndWith('\nimport "./dep-tokens.ts";')
  })

  it('creates cold directories before writing configuration and CSS', async () => {
    await register({ engine: 'wasm' })
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
    await register({ engine: 'wasm' })
    expect(wasm.getCss(null, false)).toContain('#123456')
    expect(mkdir).not.toHaveBeenCalled()
  })

  it('fails registration when reading configuration fails', async () => {
    exists.mockReturnValue(true)
    const error = new Error('unreadable fixture config')
    readConfig.mockRejectedValue(error)
    const logged = spyOn(console, 'error').mockReturnValue(undefined)
    try {
      await expect(register({ engine: 'wasm' })).rejects.toBe(error)
      hooks.expectedErrors.add(error)
      expect(logged).not.toHaveBeenCalled()
    } finally {
      logged.mockRestore()
    }
  })

  it('fails registration when the theme cannot be registered', async () => {
    exists.mockReturnValue(true)
    readConfig.mockResolvedValue(
      '{"theme":{"colors":{"default":{"card":"#fff"}},"shadows":{"default":{"card":"0 1px #000"}}}}',
    )
    const clash = new Error('clash')
    const registration = spyOn(wasm, 'registerTheme').mockImplementationOnce(
      () => {
        throw clash
      },
    )
    try {
      await expect(register({ engine: 'wasm' })).rejects.toBe(clash)
      hooks.expectedErrors.add(clash)
      expect(registration).toHaveBeenCalledTimes(1)
    } finally {
      registration.mockRestore()
    }
  })

  it('reports a failed reload of a changed Tailwind CSS and clears the theme', async () => {
    const tailwind = join(root, 'reload-tailwind.css')
    await Bun.write(tailwind, '@theme { --color-brand: #0af; }')
    const listeners: (() => void)[] = []
    const watching = spyOn(fs, 'watch').mockImplementation(((
      _file: string,
      _options: object,
      listener: () => void,
    ) => {
      listeners.push(listener)
      return { on() {}, close() {} }
    }) as unknown as typeof fs.watch)
    exists.mockReturnValue(true)
    readConfig.mockResolvedValueOnce(
      JSON.stringify({
        tailwind: { css: tailwind },
        theme: { colors: { default: { primary: '#123456' } } },
      }),
    )
    const error = new Error('broken devup.json')
    readConfig.mockRejectedValueOnce(error)
    const logged = spyOn(console, 'error').mockReturnValue(undefined)
    try {
      await register({ engine: 'wasm' })
      expect(wasm.getDefaultTheme()).toBe('default')
      expect(listeners).toHaveLength(1)

      listeners[0]!()
      for (let tick = 0; tick < 100 && logged.mock.calls.length === 0; tick++)
        await Bun.sleep(1)

      expect(logged).toHaveBeenCalledWith(error)
      expect(wasm.getDefaultTheme()).toBeUndefined()
    } finally {
      watching.mockRestore()
      logged.mockRestore()
    }
  })

  it('resolves generated stylesheets and loads their runtime placeholder', async () => {
    await register({ engine: 'wasm' })
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
    ['mts', 'ts'],
    ['cts', 'ts'],
    ['jsx', 'jsx'],
    ['mjs', 'js'],
    ['js', 'js'],
    ['cjs', 'js'],
    ['mts', 'ts'],
    ['cts', 'ts'],
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
      await register({ engine: 'wasm' })
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

  describe('included packages', () => {
    function sourceFilter() {
      const hook = loads.find(({ options }) => options.namespace === undefined)
      if (!hook) throw new Error('No source load hook registered')
      return hook.options.filter
    }

    it('compiles the published .js of an included package', async () => {
      const path = join(root, 'node_modules', 'my-lib', 'dist', 'index.js')
      await Bun.write(
        path,
        "import { css } from '@devup-ui/react'; export const className = css({ color: 'navy' })",
      )
      await register({ engine: 'wasm', include: ['my-lib'] })
      const result = await load(path)
      if (!result || !('contents' in result))
        throw new Error('Expected transformed source contents')
      expect(result.loader).toBe('js')
      expect(result.contents).not.toContain('css(')
      expect(writeSync).toHaveBeenCalledWith(
        expect.stringContaining('devup-ui.css'),
        expect.stringContaining('navy'),
        'utf-8',
      )
    })

    it.each([
      ['my-lib', '/app/node_modules/my-lib/index.js'],
      ['my-lib', 'C:\\app\\node_modules\\my-lib\\dist\\index.js'],
      [
        'my-lib',
        '/app/node_modules/.bun/my-lib@1.0.0/node_modules/my-lib/index.js',
      ],
      [
        'my-lib',
        '/app/node_modules/.pnpm/my-lib@1.0.0/node_modules/my-lib/index.js',
      ],
      ['@scope/ui', '/app/node_modules/@scope/ui/dist/index.js'],
      ['@scope/ui', 'C:\\app\\node_modules\\@scope\\ui\\index.js'],
      ['foo.bar', '/app/node_modules/foo.bar/index.js'],
      ['foo+bar', '/app/node_modules/foo+bar/index.js'],
      ['my-lib', '/app/node_modules/my-lib/dist/index.cjs'],
      ['@scope/ui', 'C:\\app\\node_modules\\@scope\\ui\\index.cjs'],
    ])('loads .js of include %s at %s', async (name, path) => {
      await register({ engine: 'wasm', include: [name] })
      expect(sourceFilter().test(path)).toBe(true)
    })

    it.each([
      ['my-lib', '/app/node_modules/my-lib-extra/index.js'],
      ['my-lib', '/app/node_modules/@scope/my-lib/index.js'],
      ['my-lib', '/app/node_modules/my-lib/node_modules/dep/index.js'],
      ['my-lib', '/app/node_modules/other/my-lib/index.js'],
      ['my-lib', '/app/node_modules/my-lib/node_modules/dep/index.cjs'],
      ['my-lib', '/app/node_modules/my-lib/index.json'],
      ['foo.bar', '/app/node_modules/fooXbar/index.js'],
      ['@scope/ui', '/app/node_modules/@scope/ui-kit/index.js'],
    ])('leaves .js outside include %s at %s to Bun', async (name, path) => {
      await register({ engine: 'wasm', include: [name] })
      expect(sourceFilter().test(path)).toBe(false)
    })

    it('loads no package .js but Devup UI without include', async () => {
      await register({ engine: 'wasm' })
      expect(sourceFilter().test('/app/node_modules/my-lib/index.js')).toBe(
        false,
      )
      expect(sourceFilter().test('/app/node_modules/my-lib/index.cjs')).toBe(
        false,
      )
      expect(
        sourceFilter().test('/app/node_modules/@devup-ui/react/dist/index.js'),
      ).toBe(true)
      expect(sourceFilter().test('/app/src/App.tsx')).toBe(true)
      // Project JavaScript is loaded whether or not a package is included
      expect(sourceFilter().test('/app/my-lib/index.js')).toBe(true)
    })

    it('runs the CommonJS .js of an included package under the runtime', async () => {
      const path = join(root, 'node_modules', 'cjs-lib', 'index.js')
      await Bun.write(
        path,
        "const { css } = require('@devup-ui/react')\nmodule.exports = { className: css({ color: 'maroon' }) }",
      )
      await register({ engine: 'wasm', include: ['cjs-lib'] })
      expect(sourceFilter().test(path)).toBe(true)
      const result = await load(path)
      if (!result || !('contents' in result))
        throw new Error('Expected transformed source contents')
      expect(result.loader).toBe('js')
      expect(result.contents).not.toContain('css(')
      expect(require.cache[path]?.exports).toEqual({
        className: expect.any(String),
      })
      expect(writeSync).toHaveBeenCalledWith(
        expect.stringContaining('devup-ui.css'),
        expect.stringContaining('maroon'),
        'utf-8',
      )
    })
  })

  it('returns source without style imports unchanged', async () => {
    const path = join(root, 'plain.ts')
    await Bun.write(path, 'export const value = 42')
    await register({ engine: 'wasm' })
    expect(await load(path)).toEqual({
      contents: 'export const value = 42',
      loader: 'ts',
    })
    expect(writeSync).not.toHaveBeenCalled()
  })

  it('writes the runtime sheet only when a module changes it', async () => {
    const first = join(root, 'sheet-first.ts')
    const same = join(root, 'sheet-same.ts')
    const style =
      "import { css } from '@devup-ui/react'; export const className = css({ color: 'olive' })"
    await Bun.write(first, style)
    await Bun.write(same, style)
    await register({ engine: 'wasm' })

    await load(first)
    expect(writeSync).toHaveBeenCalledTimes(1)
    expect(writeSync).toHaveBeenCalledWith(
      expect.stringContaining('devup-ui.css'),
      wasm.getCss(null, false),
      'utf-8',
    )

    // The same file again, and another file with only styles the sheet holds
    await load(first)
    await load(same)
    expect(writeSync).toHaveBeenCalledTimes(1)
  })

  it('propagates source read errors instead of returning partial output', async () => {
    await register({ engine: 'wasm' })
    await expect(load(join(root, 'missing.ts'))).rejects.toThrow()
  })

  it('propagates build errors through the actual callback', async () => {
    const path = join(root, 'invalid.ts')
    await Bun.write(
      path,
      "import { css } from '@devup-ui/react'; export const styles = css({ color: window.name })",
    )
    await register({ engine: 'wasm' })
    await expect(load(path)).rejects.toThrow(/invalid\.ts:\d+:\d+:/)
  })
})

describe('CommonJS under the runtime', () => {
  async function loadSource(name: string, source: string) {
    const path = join(root, name)
    await Bun.write(path, source)
    await register({ engine: 'wasm' })
    return { path, result: await load(path) }
  }

  /** Imports the ES module the plugin hands Bun in place of CommonJS */
  async function importResult(name: string, result: unknown) {
    if (
      typeof result !== 'object' ||
      result === null ||
      !('contents' in result) ||
      typeof result.contents !== 'string'
    )
      throw new Error('Expected source contents')
    expect(result).toEqual({ contents: result.contents, loader: 'js' })
    const path = join(root, `${name}.mjs`)
    await Bun.write(path, result.contents)
    const module: Record<string, unknown> = await import(path)
    return module
  }

  it('runs CommonJS that uses Devup UI and exports what it assigns', async () => {
    const { path, result } = await loadSource(
      'styled.cjs',
      "const { css } = require('@devup-ui/react')\nmodule.exports = { cls: css({ color: 'navy' }), file: __filename }",
    )
    const exported = require.cache[path]?.exports
    expect(exported).toEqual({ cls: expect.any(String), file: path })
    expect({ ...(await importResult('styled', result)) }).toEqual({
      'module.exports': exported,
      default: exported,
      cls: exported.cls,
      file: path,
    })
    expect(writeSync).toHaveBeenCalledWith(
      expect.stringContaining('devup-ui.css'),
      expect.stringContaining('navy'),
      'utf-8',
    )
  })

  it('strips TypeScript from CommonJS without fixing its path', async () => {
    const { result } = await loadSource(
      'typed.cts',
      'const directory: string = __dirname\nexport = { directory, env: process.env.NODE_ENV, defined: typeof __filename }',
    )
    expect({ ...(await importResult('typed', result)) }).toMatchObject({
      default: {
        directory: root,
        env: process.env.NODE_ENV,
        defined: 'string',
      },
    })
  })

  it('exports functions, primitives and null as Bun does', async () => {
    const callable = await loadSource(
      'callable.cjs',
      "module.exports = function named() { return 'called' }\nmodule.exports.extra = 1",
    )
    const module = await importResult('callable', callable.result)
    expect(module.default).toBe(module['module.exports'])
    expect(module.extra).toBe(1)
    expect(Object.keys(module).sort()).toEqual([
      'default',
      'extra',
      'module.exports',
    ])
    for (const [name, value] of [
      ['text', 'text'],
      ['nothing', null],
    ] as const) {
      const { result } = await loadSource(
        `${name}.cjs`,
        `module.exports = ${JSON.stringify(value)}`,
      )
      expect({ ...(await importResult(name, result)) }).toEqual({
        'module.exports': value,
        default: value,
      })
    }
  })

  it('takes the default export of a compiled ES module', async () => {
    const { result } = await loadSource(
      'compiled.cjs',
      "Object.defineProperty(exports, '__esModule', { value: true })\nexports.default = 'main'\nexports.named = 'named'",
    )
    const module = await importResult('compiled', result)
    expect(module.default).toBe('main')
    expect(module.named).toBe('named')
  })

  it('runs a module again when it is loaded again', async () => {
    const { path } = await loadSource('again.cjs', 'exports.at = Math.random()')
    const first = require.cache[path]
    await load(path)
    expect(require.cache[path]).not.toBe(first)
    expect(require.cache[path]?.loaded).toBe(true)
  })

  it('drops a module that throws from the require cache', async () => {
    const path = join(root, 'throws.cjs')
    await Bun.write(path, "exports.before = 1\nthrow new Error('boom')")
    await register({ engine: 'wasm' })
    await expect(load(path)).rejects.toThrow('boom')
    expect(require.cache[path]).toBeUndefined()
  })

  for (const [name, source, loader] of [
    ['module-syntax.cjs', "export const kind = 'module'", 'js'],
    ['top-level-await.cjs', 'module.exports = await Promise.resolve(1)', 'js'],
    ['broken.cts', 'module.exports = (', 'ts'],
  ] as const)
    it(`leaves ${name} to Bun`, async () => {
      const { result } = await loadSource(name, source)
      expect(result).toEqual({ contents: source, loader })
    })

  it('hands CommonJS to Bun.build, which bundles it', async () => {
    const entry = join(root, 'bundled-entry.mjs')
    await Bun.write(
      join(root, 'bundled.cjs'),
      "const { css } = require('@devup-ui/react')\nmodule.exports = { cls: css({ color: 'teal' }) }",
    )
    await Bun.write(
      join(root, 'bundled.cts'),
      "import { css } from '@devup-ui/react'\nexport = { cls: css({ color: 'olive' }) }",
    )
    await Bun.write(
      entry,
      "export { default as cjs } from './bundled.cjs'\nexport { default as cts } from './bundled.cts'",
    )
    const result = await Bun.build({
      entrypoints: [entry],
      target: 'bun',
      external: ['@devup-ui/react'],
      outdir: join(root, 'bundled'),
      plugins: [DevupUI({ engine: 'wasm' })],
    })
    expect(result.success).toBe(true)
    const css = await Promise.all(
      result.outputs
        .filter((output) => output.path.endsWith('.css'))
        .map((output) => output.text()),
    )
    expect(css.join('')).toMatch(/color:\s*teal/)
    expect(css.join('')).toMatch(/color:\s*olive/)
    const output = result.outputs.find(({ path }) => path.endsWith('.js'))
    if (!output) throw new Error('Expected a bundle')
    // Run the bundle in its own process, so its code stays out of coverage
    const child = Bun.spawnSync(
      [
        process.execPath,
        '-e',
        `console.log(JSON.stringify(await import(${JSON.stringify(output.path)})))`,
      ],
      { cwd: root, stdout: 'pipe', stderr: 'pipe' },
    )
    expect(child.stderr.toString()).toBe('')
    expect(JSON.parse(child.stdout.toString())).toEqual({
      cjs: { cls: expect.any(String) },
      cts: { cls: expect.any(String) },
    })
  })

  it('keeps exports through the registered runtime plugin', async () => {
    const required = join(root, 'required.cjs')
    const imported = join(root, 'imported.cts')
    await Bun.write(required, "exports.kind = 'required'")
    await Bun.write(imported, "export = { kind: 'imported' as string }")
    expect(createRequire(import.meta.url)(required)).toEqual({
      kind: 'required',
    })
    const module: Record<string, unknown> = await import(imported)
    expect(module.default).toEqual({ kind: 'imported' })
    expect(module.kind).toBe('imported')
  })
})

describe('project root', () => {
  /** Run setup as `Bun.build` does, with `root` in its config */
  async function setupBuild(buildRoot: string | undefined) {
    const ends: (() => void)[] = []
    const builder = {
      config: { entrypoints: [], root: buildRoot },
      onStart: () => builder,
      onEnd: (end: () => void) => {
        ends.push(end)
        return builder
      },
      onResolve: () => builder,
      onLoad: () => builder,
    }
    try {
      await DevupUI({ engine: 'wasm' }).setup(
        builder as unknown as Parameters<
          ReturnType<typeof DevupUI>['setup']
        >[0],
      )
    } finally {
      // End the build session setup began
      for (const end of ends) end()
    }
    return write.mock.calls.map(([path]) => String(path))
  }

  it('writes df under the root of Bun.build', async () => {
    const buildRoot = nodePath.resolve('apps', 'web')
    const written = await setupBuild(buildRoot)
    expect(written).toContain(join(buildRoot, 'df', '.gitignore'))
    expect(written).toContain(join(buildRoot, 'df', 'devup-ui', 'devup-ui.css'))
  })

  it('resolves imports with the tsconfig.json of the root of Bun.build when started from a parent directory', async () => {
    // The tsconfig.json under the root is real here
    exists.mockImplementation(realExists)
    const buildRoot = fs.mkdtempSync(join(tmpdir(), 'devup-ui-bun-root-'))
    using setModuleResolver = spyOn(
      pluginUtils.loadCompiler('wasm'),
      'setModuleResolver',
    ).mockReturnValue(undefined)
    try {
      await Bun.write(
        join(buildRoot, 'tsconfig.json'),
        JSON.stringify({
          compilerOptions: { paths: { '@root-fixture/*': ['./src/*'] } },
        }),
      )
      await Bun.write(join(buildRoot, 'src', 'tokens.ts'), 'export const a = 1')
      await setupBuild(buildRoot)
      const resolveModule = setModuleResolver.mock.calls.at(-1)?.[0]
      if (typeof resolveModule !== 'function')
        throw new Error('No module resolver')
      expect(
        resolveModule(
          '@root-fixture/tokens',
          join(buildRoot, 'src', 'App.tsx'),
        ),
      ).toEqual({
        path: join(buildRoot, 'src', 'tokens.ts'),
        code: 'export const a = 1',
      })
    } finally {
      fs.rmSync(buildRoot, { recursive: true, force: true })
    }
  })

  it('keeps writing df in the cwd without a root', async () => {
    const written = await setupBuild(undefined)
    expect(written).toContain(join('df', '.gitignore'))
    expect(written).toContain(
      nodePath.resolve('df', 'devup-ui', 'devup-ui.css'),
    )
  })

  it('falls back to the devup.json of the cwd with a warning', async () => {
    const warn = spyOn(console, 'warn').mockReturnValue(undefined)
    const cwdFile = nodePath.resolve('devup.json')
    exists.mockImplementation((path) => String(path) === cwdFile)
    try {
      const buildRoot = nodePath.resolve('apps', 'bun-fallback')
      const written = await setupBuild(buildRoot)
      expect(readConfig).toHaveBeenCalledWith(cwdFile, 'utf-8')
      expect(written).toContain(join(buildRoot, 'df', '.gitignore'))
      expect(warn).toHaveBeenCalledTimes(1)
      expect(String(warn.mock.calls[0]?.[0])).toContain('devupFile')
    } finally {
      warn.mockRestore()
    }
  })
})
