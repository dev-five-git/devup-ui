import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import {
  BuildGeneration,
  createCompatTypes,
  createNodeModulesExcludeRegex,
  mergeImportAliases,
} from '@devup-ui/plugin-utils'
import * as webpackPluginModule from '@devup-ui/webpack-plugin'
import { describe, expect, it, mock, spyOn } from 'bun:test'
import type { NextConfig } from 'next'

import { DevupUI, reloadTurboSetupModuleForTesting } from '../plugin'
import { readCoordinatorState } from '../state'
import { setWebpackPluginForTesting } from '../wasm'
import { box, installProjectHooks, makeProject } from './project'
import { installTurboHarness } from './turbo-harness'

installProjectHooks()
const harness = installTurboHarness()

type NextWebpackConfig = Parameters<
  NonNullable<ReturnType<typeof DevupUI>['webpack']>
>[0]
type NextWebpackContext = Parameters<
  NonNullable<ReturnType<typeof DevupUI>['webpack']>
>[1]

const SOURCE_RULE = '*.{tsx,ts,jsx,js,mjs,mts,cts,cjs}'
const page = box('bg="red"')

function project(extra: Record<string, string> = {}): string {
  const root = makeProject({ 'src/app/page.tsx': page, ...extra })
  process.chdir(root)
  return root
}

function development(): void {
  process.env.NODE_ENV = 'development'
}

describe('webpack', () => {
  type ContextOverrides = Readonly<
    Partial<Omit<NextWebpackContext, 'config'>>
  > & { readonly config?: object }

  function run(
    options: Parameters<typeof DevupUI>[1],
    contexts: readonly ContextOverrides[],
    userWebpack?: ReturnType<typeof mock>,
  ) {
    delete process.env.TURBOPACK
    type PluginArguments = ConstructorParameters<
      typeof webpackPluginModule.DevupUIWebpackPlugin
    >
    const spy = mock((..._args: PluginArguments) => {})
    class CapturingPlugin extends webpackPluginModule.DevupUIWebpackPlugin {
      constructor(...args: PluginArguments) {
        super(...args)
        spy(...args)
      }
    }
    setWebpackPluginForTesting({
      ...webpackPluginModule,
      DevupUIWebpackPlugin: CapturingPlugin,
    })
    const configs: NextWebpackConfig[] = []
    const inputs: NextWebpackContext[] = []
    const results: NextWebpackConfig[] = []
    try {
      const ret = DevupUI({ webpack: userWebpack }, options)
      for (const context of contexts) {
        const config = { plugins: [] } as unknown as NextWebpackConfig
        const input = {
          buildId: 'tmpBuildId',
          config: {},
          ...context,
        } as NextWebpackContext
        configs.push(config)
        inputs.push(input)
        results.push(ret.webpack!(config, input))
      }
      return { spy, configs, inputs, results }
    } finally {
      setWebpackPluginForTesting(undefined)
    }
  }

  it('applies the webpack plugin with the production cache directory', () => {
    const { spy } = run({}, [{}])

    expect(spy).toHaveBeenCalledWith(
      {
        cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
      },
      expect.objectContaining({ complete: true }),
    )
    spy.mockRestore()
  })

  it('applies the webpack plugin in dev', () => {
    const { spy } = run({}, [{ dev: true }])

    expect(spy).toHaveBeenCalledWith(
      {
        cssDir: resolve('df', 'devup-ui_tmpBuildId'),
        watch: true,
      },
      expect.objectContaining({ complete: true }),
    )
    spy.mockRestore()
  })

  it('forwards the options when no caller webpack function is configured', () => {
    // Given
    const options = { package: 'new-package' }
    // When
    const { spy, configs, results } = run(options, [{}])
    try {
      // Then
      expect(spy).toHaveBeenCalledWith(
        {
          package: 'new-package',
          cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
        },
        expect.objectContaining({ complete: true }),
      )
      expect(results[0]).toBe(configs[0])
    } finally {
      spy.mockRestore()
    }
  })

  it('forwards options and preserves order, arguments and return when a caller webpack function is configured', () => {
    // Given
    const returned = { plugins: [], name: 'caller-result' }
    const pluginCounts: number[] = []
    const webpack = mock(
      (config: NextWebpackConfig, _context: NextWebpackContext) => {
        pluginCounts.push(config.plugins.length)
        return returned
      },
    )
    // When
    const { spy, configs, inputs, results } = run(
      { package: 'new-package' },
      [{}],
      webpack,
    )
    try {
      // Then
      expect(spy).toHaveBeenCalledWith(
        {
          package: 'new-package',
          cssDir: resolve('.next/cache', 'devup-ui_tmpBuildId'),
        },
        expect.objectContaining({ complete: true }),
      )
      expect(pluginCounts).toEqual([1])
      expect(webpack).toHaveBeenCalledTimes(1)
      expect(webpack.mock.calls[0]?.[0]).toBe(configs[0])
      expect(webpack.mock.calls[0]?.[1]).toBe(inputs[0])
      expect(results[0]).toBe(returned)
    } finally {
      spy.mockRestore()
    }
  })

  it('shares a real owner when one wrapper constructs production server and client configs', () => {
    // Given
    const config = {}
    // When
    const { spy } = run({}, [
      { config, dev: false, isServer: true },
      { config, dev: false, isServer: false },
    ])
    try {
      // Then
      expect(spy).toHaveBeenCalledTimes(2)
      const server = spy.mock.calls[0]?.[1]
      const client = spy.mock.calls[1]?.[1]
      expect(server?.owner).toBeInstanceOf(BuildGeneration)
      expect(client?.owner).toBe(server?.owner)
      expect(server?.complete).toBe(false)
      expect(client?.complete).toBe(true)
    } finally {
      spy.mockRestore()
    }
  })

  it('marks a real owner complete when a wrapper constructs a development server config', () => {
    // Given
    const context = { config: {}, dev: true, isServer: true }
    // When
    const { spy } = run({}, [context])
    try {
      // Then
      expect(spy).toHaveBeenCalledTimes(1)
      const binding = spy.mock.calls[0]?.[1]
      expect(binding?.owner).toBeInstanceOf(BuildGeneration)
      expect(binding?.complete).toBe(true)
    } finally {
      spy.mockRestore()
    }
  })

  it('isolates real owners when independent wrappers receive the same public context', () => {
    // Given
    const context = {
      config: {},
      dev: false,
      isServer: true,
      buildId: 'fixed',
    }
    const first = run({}, [context])
    const firstOwner = first.spy.mock.calls[0]?.[1]?.owner
    first.spy.mockRestore()
    // When
    const { spy } = run({}, [context])
    try {
      // Then
      const secondOwner = spy.mock.calls[0]?.[1]?.owner
      expect(firstOwner).toBeInstanceOf(BuildGeneration)
      expect(secondOwner).toBeInstanceOf(BuildGeneration)
      expect(secondOwner).not.toBe(firstOwner)
    } finally {
      spy.mockRestore()
    }
  })
})

describe('turbopack setup', () => {
  it('applies every option explicitly, with absolute paths and one identity', () => {
    const root = project({
      'cfg/devup.json': JSON.stringify({
        extends: ['./base.json'],
        theme: { colors: { dark: { primary: '#000' } } },
      }),
      'cfg/base.json': JSON.stringify({
        theme: { colors: { light: { primary: '#fff' } } },
      }),
    })
    const config: NextConfig = { env: { EXISTING: 'value' } }

    const result = DevupUI(config, {
      package: '@devup-ui/react',
      distDir: 'out',
      cssDir: 'out/styles',
      singleCss: true,
      devupFile: 'cfg/devup.json',
      include: ['@acme/ui'],
      prefix: 'du-',
      shorthands: { insetX: ['left', 'right'] },
      atomHoist: 2,
      debug: true,
      importAliases: { 'styled-components': false },
    })

    expect(result).toBe(config)
    const [start] = harness.starts
    expect(start).toMatchObject({
      package: '@devup-ui/react',
      cssDir: join(root, 'out', 'styles'),
      singleCss: true,
      projectRoot: root,
      watch: false,
      devupFile: join(root, 'cfg', 'devup.json'),
      sourceRoots: ['src', 'app', 'pages'].map((dir) => join(root, dir)),
      sourceMap: false,
      importAliases: {
        '@emotion/react': null,
        '@emotion/styled': 'styled',
        '@vanilla-extract/css': null,
      },
      expectedBaseFiles: ['src/app/page.tsx'],
      prewarmedFiles: ['src/app/page.tsx'],
    })
    expect(start!.wasm.isDebug()).toBe(true)
    expect(start!.wasm.getPrefix()).toBe('du-')
    expect(start!.identity?.project).toBe(root)
    expect(start!.coordinatorPortFile).toBe(
      join(
        root,
        'out',
        '.devup',
        start!.optionsKey!,
        'sessions',
        `${process.pid}-${start!.identity?.token}`,
        'endpoint.json',
      ),
    )
    expect(result.env).toEqual({
      EXISTING: 'value',
      DEVUP_UI_DEFAULT_THEME: 'light',
    })
    expect(existsSync(join(root, 'out', 'styles', 'devup-ui.css'))).toBe(true)
  })

  it('applies the defaults explicitly too', () => {
    project()

    DevupUI({})

    const [start] = harness.starts
    expect(start!.wasm.isDebug()).toBe(false)
    expect(start!.wasm.getPrefix()).toBeUndefined()
    expect(start).toMatchObject({
      package: '@devup-ui/react',
      singleCss: false,
      watch: false,
      sourceMap: false,
      canonicalMap: {},
      expectedBaseFiles: ['src/app/page.tsx'],
    })
    expect(start!.stateFile).toBeUndefined()
    expect(start!.revisionFile).toBeUndefined()
    expect(start!.configureWasm).toBeFunction()
    expect(start!.createEngine).toBeFunction()
    expect(start!.createEngine!()).not.toBe(start!.wasm)
  })

  it('configures every engine it builds, including the rebuilds', () => {
    project()
    DevupUI({}, { debug: true, prefix: 'du-' })
    const [start] = harness.starts

    const fresh = start!.createEngine!()
    start!.configureWasm!(fresh)

    expect(fresh.isDebug()).toBe(true)
    expect(fresh.getPrefix()).toBe('du-')
  })

  it('hands the loaders one identity, endpoint and the config files', () => {
    const root = project({
      'devup.json': JSON.stringify({ extends: ['./base.json'] }),
      'base.json': JSON.stringify({
        theme: { colors: { default: { primary: '#fff' } } },
      }),
    })

    const result = DevupUI({}, { include: ['@acme/ui'] })

    const [start] = harness.starts
    const common = {
      coordinatorPortFile: start!.coordinatorPortFile,
      coordinatorIdentity: start!.identity,
      projectRoot: root,
      themeFiles: [join(root, 'devup.json'), join(root, 'base.json')],
      requestTimeoutMs: 120_000,
      watch: false,
      themeFile: join(root, 'devup.json'),
      theme: { colors: { default: { primary: '#fff' } } },
      defaultSheet: {},
      defaultClassMap: {},
      defaultFileMap: {},
    }
    expect(result.turbopack?.rules).toEqual({
      './df/devup-ui/*.css': [
        {
          loader: '@devup-ui/next-plugin/css-loader',
          options: expect.objectContaining(common),
        },
      ],
      [SOURCE_RULE]: {
        loaders: [
          {
            loader: '@devup-ui/next-plugin/loader',
            options: expect.objectContaining({
              ...common,
              package: '@devup-ui/react',
              cssDir: join(root, 'df', 'devup-ui'),
              singleCss: false,
              importAliases: {
                '@emotion/react': null,
                '@emotion/styled': 'styled',
                '@vanilla-extract/css': null,
                'styled-components': 'styled',
              },
            }),
          },
        ],
        condition: {
          not: {
            path: createNodeModulesExcludeRegex(['@acme/ui']),
          },
        },
      },
    })
    expect(result.turbopack?.rules?.[SOURCE_RULE]).not.toHaveProperty(
      'loaders.0.options.revisionFile',
    )
  })

  it('keeps existing Turbopack rules', () => {
    project()

    const result = DevupUI({ turbopack: { rules: { '*.svg': ['svgr'] } } })

    expect(result.turbopack?.rules).toHaveProperty(['*.svg'])
    expect(result.turbopack?.rules).toHaveProperty([SOURCE_RULE])
  })

  it('matches every module extension Turbopack compiles', () => {
    project()
    const rule = Object.keys(DevupUI({}).turbopack?.rules ?? {}).find((key) =>
      key.startsWith('*.'),
    )

    expect(rule).toBe('*.{tsx,ts,jsx,js,mjs,mts,cts,cjs}')
  })

  it('starts development with a state file and a revision file', () => {
    development()
    const root = project()
    const seen: { state: boolean; revision: boolean }[] = []
    harness.onStart = (options) =>
      seen.push({
        state: existsSync(options.stateFile!),
        revision: existsSync(options.revisionFile!),
      })

    const result = DevupUI({})

    const [start] = harness.starts
    expect(start).toMatchObject({
      watch: true,
      sourceMap: true,
      stateFile: join(
        root,
        'df',
        '.devup',
        start!.optionsKey!,
        'snapshot.json',
      ),
      revisionFile: join(
        root,
        'df',
        '.devup',
        start!.optionsKey!,
        'sessions',
        `${process.pid}-${start!.identity?.token}`,
        'revision',
      ),
    })
    expect(seen).toEqual([{ state: true, revision: false }])
    const loader = JSON.stringify(result.turbopack?.rules)
    expect(loader).toContain('"watch":true')
    expect(loader).toContain(JSON.stringify(start!.revisionFile).slice(1, -1))
    expect(result.compiler).toBeUndefined()
  })

  it('commits the initial development state before the coordinator can publish', () => {
    development()
    project({ 'src/app/b/page.tsx': box('bg="blue"') })
    let committed: ReturnType<typeof readCoordinatorState>
    harness.onStart = (options) => {
      committed = readCoordinatorState(options.stateFile!, options.optionsKey!)
    }

    DevupUI({})

    expect(committed).toMatchObject({
      version: 1,
      revision: 0,
      fileMap: { 'src/app/b/page.tsx': 0, 'src/app/page.tsx': 1 },
    })
    expect(committed?.inputs.map((input) => input.filename)).toEqual([
      'src/app/b/page.tsx',
      'src/app/page.tsx',
    ])
  })

  it('writes the generated files once and keeps the CSS placeholder content stable', () => {
    const root = project({
      'devup.json': JSON.stringify({
        theme: { colors: { default: { primary: '#fff' } } },
      }),
    })

    DevupUI({})
    DevupUI({})

    const dist = join(root, 'df')
    expect(readFileSync(join(dist, '.gitignore'), 'utf-8')).toBe('*')
    expect(readFileSync(join(dist, 'compat.d.ts'), 'utf-8')).toBe(
      createCompatTypes(mergeImportAliases()),
    )
    expect(readFileSync(join(dist, 'theme.d.ts'), 'utf-8')).toContain('primary')
    expect(readFileSync(join(dist, 'devup-ui', 'devup-ui.css'), 'utf-8')).toBe(
      '',
    )
  })

  it('does not rewrite the CSS placeholder that Turbopack already tracks', () => {
    const root = project()
    DevupUI({})
    const placeholder = join(root, 'df', 'devup-ui', 'devup-ui.css')
    writeFileSync(placeholder, '/* kept */')

    DevupUI({})

    expect(readFileSync(placeholder, 'utf-8')).toBe('/* kept */')
  })

  it('writes no theme declaration for an app without a theme', () => {
    const root = project()

    DevupUI({})

    expect(existsSync(join(root, 'df', 'theme.d.ts'))).toBe(false)
  })

  it('names the file when the devup config cannot be read', () => {
    const root = project({ 'devup.json': '{ not json' })

    expect(() => DevupUI({})).toThrow(
      `${join(root, 'devup.json')}:1:1: devup config cannot use \`JSON\` at build time`,
    )
    expect(harness.starts).toHaveLength(0)
  })

  it('takes source maps from Next in production', () => {
    project()

    DevupUI({ productionBrowserSourceMaps: true })
    DevupUI({})

    expect(harness.starts.map((start) => start.sourceMap)).toEqual([
      true,
      false,
    ])
  })

  it('registers the exit hooks that drain and close the coordinator', async () => {
    project()

    DevupUI({})

    expect(Object.keys(harness.handlers).sort()).toEqual(['beforeExit', 'exit'])
    await harness.handlers.beforeExit![0]!()
    expect(harness.handles[0]!.drain).toHaveBeenCalledTimes(1)
    expect(harness.handles[0]!.close).toHaveBeenCalledTimes(1)
  })

  it('reports the setup profile when asked', () => {
    project()
    process.env.DEVUP_UI_PROFILE = '1'
    const info = spyOn(console, 'info').mockImplementation(() => {})

    try {
      DevupUI({}, { singleCss: true })

      const phases = info.mock.calls
        .map(([line]) => String(line).replace('[devup-ui:profile] ', ''))
        .map((line) => JSON.parse(line))
      expect(phases.map((entry) => entry.phase)).toEqual([
        'next.graph',
        'next.prewarm',
        'next.setup',
      ])
      expect(phases[2]).toMatchObject({
        cacheHit: false,
        pid: process.pid,
        prewarmedFiles: 1,
        singleCss: true,
        watch: false,
      })
    } finally {
      info.mockRestore()
    }
  })
})

describe('turbopack apps in one process', () => {
  it('gives an app without a theme the default theme, never another app’s', () => {
    project({
      'devup.json': JSON.stringify({
        theme: {
          colors: { light: { primary: '#fff' }, dark: { primary: '#000' } },
        },
      }),
    })
    const themed = DevupUI({})
    project()
    const plain = DevupUI({})

    expect(themed.env).toEqual({ DEVUP_UI_DEFAULT_THEME: 'light' })
    expect(plain.env).toEqual({ DEVUP_UI_DEFAULT_THEME: 'default' })
    expect(process.env.DEVUP_UI_DEFAULT_THEME).toBeUndefined()
  })

  it('keeps the prefix, theme, debug mode and state of two apps apart', () => {
    project({
      'devup.json': JSON.stringify({
        theme: { colors: { default: { primary: 'red' } } },
      }),
    })
    DevupUI({}, { prefix: 'a-', debug: true })
    const rootB = project({ 'src/app/b/page.tsx': box('bg="blue"') })
    DevupUI({})

    const [a, b] = harness.starts
    expect(a!.wasm).not.toBe(b!.wasm)
    expect(a!.wasm.getPrefix()).toBe('a-')
    expect(b!.wasm.getPrefix()).toBeUndefined()
    expect(a!.wasm.isDebug()).toBe(true)
    expect(b!.wasm.isDebug()).toBe(false)
    expect(a!.wasm.getCss(undefined, false)).toContain('--primary:red')
    expect(b!.wasm.getCss(undefined, false)).not.toContain('--primary')
    expect(JSON.parse(a!.wasm.exportFileMap())).toEqual({
      'src/app/page.tsx': 0,
    })
    expect(JSON.parse(b!.wasm.exportFileMap())).toEqual({
      'src/app/b/page.tsx': 0,
      'src/app/page.tsx': 1,
    })
    expect(a!.coordinatorPortFile).not.toBe(b!.coordinatorPortFile)
    expect(a!.identity?.project).not.toBe(rootB)
  })

  it('resolves everything against the root the app was set up in', () => {
    const rootA = project()
    const rootB = project()
    process.chdir(rootA)

    const result = DevupUI({}, { distDir: 'out', devupFile: 'config.json' })
    process.chdir(rootB)

    const [start] = harness.starts
    expect(start).toMatchObject({
      cssDir: join(rootA, 'out', 'devup-ui'),
      projectRoot: rootA,
      devupFile: join(rootA, 'config.json'),
    })
    expect(start!.coordinatorPortFile.startsWith(join(rootA, 'out'))).toBe(true)
    expect(Object.keys(result.turbopack?.rules ?? {})[0]).toBe(
      './out/devup-ui/*.css',
    )
    expect(JSON.stringify(result.turbopack?.rules)).not.toContain(
      JSON.stringify(rootB).slice(1, -1),
    )
  })

  it('gives every call of the same app its own session', () => {
    project()

    DevupUI({})
    DevupUI({})

    const [first, second] = harness.starts
    expect(first!.coordinatorPortFile).not.toBe(second!.coordinatorPortFile)
    expect(first!.identity?.token).not.toBe(second!.identity?.token)
    expect(first!.optionsKey).toBe(second!.optionsKey)
    expect(harness.handlers.exit).toHaveLength(2)
  })

  it('continues the names of the last development session and appends new files', () => {
    development()
    const root = project({ 'src/app/b/page.tsx': box('bg="blue"') })
    DevupUI({})
    writeFileSync(join(root, 'src/aaa.tsx'), box('bg="green"'))

    DevupUI({})

    const [first, second] = harness.starts
    expect(JSON.parse(first!.wasm.exportFileMap())).toEqual({
      'src/app/b/page.tsx': 0,
      'src/app/page.tsx': 1,
    })
    expect(JSON.parse(second!.wasm.exportFileMap())).toEqual({
      'src/app/b/page.tsx': 0,
      'src/app/page.tsx': 1,
      'src/aaa.tsx': 2,
    })
  })
})

describe('turbopack production setup handoff', () => {
  it('hands one setup to the second evaluation of the config', () => {
    const root = project({
      'devup.json': JSON.stringify({
        theme: { colors: { light: { primary: '#fff' } } },
      }),
    })
    process.env.DEVUP_UI_PROFILE = '1'
    const info = spyOn(console, 'info').mockImplementation(() => {})

    try {
      const first = DevupUI({}, { singleCss: true })
      reloadTurboSetupModuleForTesting()
      const second = DevupUI(
        { env: { EXISTING: 'value' } },
        { singleCss: true },
      )

      expect(harness.starts).toHaveLength(1)
      expect(second.turbopack?.rules).toEqual(first.turbopack?.rules)
      expect(second.env).toEqual({
        DEVUP_UI_DEFAULT_THEME: 'light',
        EXISTING: 'value',
      })
      expect(JSON.stringify(second.turbopack?.rules)).toContain(
        harness.starts[0]!.identity!.token,
      )
      expect(JSON.stringify(second.turbopack?.rules)).toContain(
        JSON.stringify(root).slice(1, -1),
      )
      expect(info).toHaveBeenCalledWith(
        expect.stringContaining('"cacheHit":true'),
      )
    } finally {
      info.mockRestore()
    }
  })

  it('does not reuse a setup for other options, config contents or its own module', () => {
    const root = project({ 'devup.json': '{}' })
    DevupUI({}, { singleCss: true })

    DevupUI({}, { singleCss: true })
    expect(harness.starts).toHaveLength(2)

    reloadTurboSetupModuleForTesting()
    DevupUI({}, { singleCss: false })
    expect(harness.starts).toHaveLength(3)

    reloadTurboSetupModuleForTesting()
    writeFileSync(join(root, 'devup.json'), '{ "theme": {} }')
    DevupUI({}, { singleCss: true })
    expect(harness.starts).toHaveLength(4)

    reloadTurboSetupModuleForTesting()
    DevupUI({}, { singleCss: true, debug: true })
    expect(harness.starts).toHaveLength(5)
  })

  it('does not reuse a setup across projects', () => {
    project()
    DevupUI({})
    reloadTurboSetupModuleForTesting()
    project()

    DevupUI({})

    expect(harness.starts).toHaveLength(2)
  })

  it('never hands a setup over in development', () => {
    development()
    project()
    DevupUI({})
    reloadTurboSetupModuleForTesting()

    DevupUI({})

    expect(harness.starts).toHaveLength(2)
  })

  it('drains the first evaluation’s coordinator from either config', async () => {
    project()
    const user = mock(async () => {})
    const first = DevupUI({ compiler: { runAfterProductionCompile: user } })
    reloadTurboSetupModuleForTesting()
    const second = DevupUI({})
    const metadata = { projectDir: '/p', distDir: '.next' }

    await second.compiler?.runAfterProductionCompile?.(metadata)
    expect(harness.handles).toHaveLength(1)
    expect(harness.handles[0]!.drain).toHaveBeenCalledTimes(1)
    expect(harness.handles[0]!.close).not.toHaveBeenCalled()

    await first.compiler?.runAfterProductionCompile?.(metadata)
    expect(harness.handles[0]!.drain).toHaveBeenCalledTimes(2)
    expect(user).toHaveBeenCalledWith(metadata)
  })
})

describe('turbopack prewarm scope', () => {
  const dynamic = `import { css } from '@devup-ui/react'\nconst v = Math.random()\nexport const c = css({ bg: v })`

  it('does not extract or fail on a dead file nothing compiles', () => {
    project({ 'src/dead/broken.tsx': dynamic })

    expect(() => DevupUI({})).not.toThrow()

    expect(harness.starts[0]!.prewarmedFiles).toEqual(['src/app/page.tsx'])
    expect(harness.starts[0]!.expectedBaseFiles).toEqual(['src/app/page.tsx'])
  })

  it('fails a production build on a reachable file with its location', () => {
    project({
      'src/app/page.tsx': `import './broken'\n${page}`,
      'src/app/broken.tsx': dynamic,
    })

    expect(() => DevupUI({})).toThrow(
      'src/app/broken.tsx:3:18: `css()` cannot use `v` at build time',
    )
    expect(harness.starts).toHaveLength(0)
  })

  it('prewarms the whole tree only on request', () => {
    project({ 'src/dead/broken.tsx': dynamic })

    expect(() => DevupUI({}, { prewarmAll: true })).toThrow(
      'src/dead/broken.tsx:3:18: `css()` cannot use `v` at build time',
    )
  })

  it('prewarms the packages the reached files import', () => {
    project({
      'src/app/page.tsx': `import 'design-system'\n${page}`,
      'node_modules/design-system/package.json': JSON.stringify({
        name: 'design-system',
        main: './index.js',
      }),
      'node_modules/design-system/index.js': 'export const x = 1',
    })

    DevupUI({}, { include: ['design-system'] })

    expect(harness.starts[0]!.prewarmedFiles).toEqual([
      'node_modules/design-system/index.js',
      'src/app/page.tsx',
    ])
  })

  it('is a finished, empty plan when the project has no compiled source', () => {
    process.chdir(makeProject())

    DevupUI({})

    const [start] = harness.starts
    expect(start!.prewarmedFiles).toEqual([])
    expect(start!.expectedBaseFiles).toEqual([])
    expect(start!.prewarmedOutputs?.size).toBe(0)
  })

  it('fails a production build with the location of a graph failure', () => {
    const root = makeProject({ src: 'not a directory' })
    process.chdir(root)

    expect(() => DevupUI({})).toThrow(
      `${join(root, 'src')}:1:1: devup-ui import graph cannot use \`buildStaticImportGraph\` at build time`,
    )
    expect(harness.starts).toHaveLength(0)
  })

  it('warns in development, names the lost guarantee and still starts', () => {
    development()
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    const root = makeProject({ src: 'not a directory' })
    process.chdir(root)

    try {
      DevupUI({})

      expect(harness.starts).toHaveLength(1)
      expect(warn.mock.calls.map(([line]) => String(line))).toEqual([
        expect.stringContaining(
          'Not guaranteed for this session: single-importer',
        ),
        expect.stringContaining(
          'Not guaranteed for this session: path-ordered',
        ),
      ])
      expect(harness.starts[0]).toMatchObject({
        expectedBaseFiles: [],
        prewarmedFiles: [],
      })
    } finally {
      warn.mockRestore()
    }
  })

  it('reports the lost graph guarantees for an unreadable included package in development', () => {
    development()
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    const root = project({
      'src/app/page.tsx': `import 'design-system'\n${page}`,
      'node_modules/design-system/package.json': '{ not json',
    })

    try {
      DevupUI({}, { include: ['design-system'] })

      expect(harness.starts[0]!.prewarmedFiles).toEqual([])
      expect(harness.starts[0]!.expectedBaseFiles).toEqual([])
      expect(String(warn.mock.calls[0]?.[0])).toContain(
        `${join(root, 'node_modules/design-system/package.json')}:1:1: Cannot load configuration:`,
      )
      expect(String(warn.mock.calls[0]?.[0])).toContain(
        'Not guaranteed for this session: single-importer collapse, atom hoisting and the deterministic completion set',
      )
    } finally {
      warn.mockRestore()
    }
  })
})
