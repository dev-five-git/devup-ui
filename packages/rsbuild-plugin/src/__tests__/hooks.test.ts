import {
  existsSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
} from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import { DevupUI } from '../plugin'

// The plugin driven through the hooks Rsbuild calls, in the order it calls
// them, with the real compiler and a real (temporary) file system: the
// sibling suite mocks every write as already done, which hides ordering.

type Setup = Parameters<ReturnType<typeof DevupUI>['setup']>[0]

interface Environment {
  name: string
  entry: Record<string, string>
}

interface TransformContext {
  code: string
  resourcePath: string
  environment: Environment
  addDependency: (path: string) => void
  addMissingDependency: (path: string) => void
}

type Transform = (context: TransformContext) => unknown

interface RspackPlugin {
  apply(compiler: FakeCompiler): void
}

type ModifyRspackConfig = (
  config: { plugins?: RspackPlugin[] },
  utils: { environment: Environment },
) => void

type Tap = (...args: unknown[]) => unknown

interface FakeCompiler {
  options: { entry: Record<string, string> }
  hooks: Record<string, { tap: (name: unknown, fn: Tap) => void }>
  rspack: { Compilation: { PROCESS_ASSETS_STAGE_REPORT: number } }
  modifiedFiles?: Set<string>
}

function fakeApi(root: string) {
  const order: string[] = []
  const transforms: { css: boolean; handler: Transform }[] = []
  const rspackConfigs: ModifyRspackConfig[] = []
  const api = {
    context: { rootPath: root },
    onCloseBuild: () => {
      order.push('onCloseBuild')
    },
    modifyRspackConfig: (fn: ModifyRspackConfig) => {
      order.push('modifyRspackConfig')
      rspackConfigs.push(fn)
    },
    onBeforeBuild: () => {
      order.push('onBeforeBuild')
    },
    modifyRsbuildConfig: () => {
      order.push('modifyRsbuildConfig')
    },
    transform: ({ test }: { test: unknown }, handler: Transform) => {
      const css = typeof test === 'string'
      const mdx = test instanceof RegExp && test.test('page.mdx')
      order.push(
        css
          ? 'transform(cssDir)'
          : mdx
            ? 'transform(mdx)'
            : 'transform(source)',
      )
      transforms.push({ css, handler })
    },
  }
  const transform = (css: boolean) => {
    const found = transforms.find((item) => item.css === css)
    if (!found) throw new Error('transform not registered')
    return found.handler
  }
  return {
    api: api as unknown as Setup,
    order,
    rspackConfigs,
    css: (context: TransformContext) => transform(true)(context),
    source: (context: TransformContext) => transform(false)(context),
  }
}

/** An Rspack compiler and compilation that run the taps they are given */
function fakeCompiler() {
  const taps = new Map<string, Tap[]>()
  const hook = (name: string) => ({
    tap: (_: unknown, fn: Tap) =>
      taps.set(name, [...(taps.get(name) ?? []), fn]),
  })
  const call = (name: string, ...args: unknown[]) =>
    (taps.get(name) ?? []).map((fn) => fn(...args))
  const compiler: FakeCompiler = {
    options: { entry: {} },
    hooks: Object.fromEntries(
      ['run', 'invalid', 'watchRun', 'thisCompilation'].map((name) => [
        name,
        hook(name),
      ]),
    ),
    rspack: { Compilation: { PROCESS_ASSETS_STAGE_REPORT: 5000 } },
  }
  const compilation = {
    hooks: Object.fromEntries(
      ['finishModules', 'processAssets', 'needAdditionalPass'].map((name) => [
        name,
        hook(name),
      ]),
    ),
    assets: { 'main.js': {} },
    deleteAsset: mock(),
  }
  return { compiler, compilation, call }
}

let root: string
const realMkdir = fsPromises.mkdir

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-rsbuild-hooks-'))
})

afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

function plugin(options: Parameters<typeof DevupUI>[0] = {}) {
  return DevupUI({
    engine: 'wasm',
    atomHoist: 0,
    distDir: join(root, 'df'),
    cssDir: join(root, 'df', 'devup-ui'),
    devupFile: join(root, 'devup.json'),
    ...options,
  })
}

const web: Environment = { name: 'web', entry: {} }
const app = (color: string) =>
  `import { Box } from '@devup-ui/react'\nexport const App = () => <Box color="${color}" />`

async function extract(
  hooks: ReturnType<typeof fakeApi>,
  code: string,
  environment = web,
) {
  return hooks.source({
    code,
    resourcePath: join(root, 'src', 'App.tsx'),
    environment,
    addDependency: () => {},
    addMissingDependency: () => {},
  })
}

/** The class name a transformed module renders */
function className(result: unknown): string {
  const code =
    typeof result === 'object' && result !== null && 'code' in result
      ? String(result.code)
      : ''
  const match = /className="([^"]+)"/.exec(code)
  if (!match) throw new Error(`no className in ${code}`)
  return match[1]
}

/** The per-file stylesheets the transforms wrote, by name */
function sheets(cssDir = join(root, 'df', 'devup-ui')) {
  return readdirSync(cssDir).filter((name) => name !== 'devup-ui.css')
}

describe('hook registration', () => {
  it('registers the hooks a build runs, in the order it runs them', async () => {
    const hooks = fakeApi(root)
    await plugin().setup(hooks.api)
    expect(hooks.order).toEqual([
      'onCloseBuild',
      'modifyRspackConfig',
      'onBeforeBuild',
      'transform(cssDir)',
      'modifyRspackConfig',
      'modifyRsbuildConfig',
      'transform(source)',
      'transform(mdx)',
    ])
  })

  it('registers only the build end when it does not extract CSS', async () => {
    const hooks = fakeApi(root)
    await plugin({ extractCss: false }).setup(hooks.api)
    expect(hooks.order).toEqual(['onCloseBuild'])
  })
})

describe('cold directories', () => {
  it('creates the CSS directory before it writes the first stylesheet into it', async () => {
    // A slow mkdir: a write started next to it, not after it, fails
    const mkdir = spyOn(fsPromises, 'mkdir').mockImplementation(
      async (...args: Parameters<typeof realMkdir>) => {
        await Bun.sleep(20)
        return realMkdir(...args)
      },
    )
    try {
      const cssDir = join(root, 'cold', 'nested', 'devup-ui')
      await plugin({ cssDir }).setup(fakeApi(root).api)
      expect(existsSync(join(cssDir, 'devup-ui.css'))).toBe(true)
      expect(mkdir).toHaveBeenCalledWith(cssDir, { recursive: true })
    } finally {
      mkdir.mockRestore()
    }
  })
})

describe('CSS revisions', () => {
  it('serves and writes the rule of a same-length edit', async () => {
    const hooks = fakeApi(root)
    await plugin().setup(hooks.api)
    const first = className(await extract(hooks, app('red')))
    const [sheet] = sheets()
    const path = join(root, 'df', 'devup-ui', sheet)
    const serve = () =>
      String(
        hooks.css({
          code: '',
          resourcePath: path,
          environment: web,
          addDependency: () => {},
          addMissingDependency: () => {},
        }),
      )
    expect(serve()).toContain(`.${first}{color:red}`)

    // `red` and `tan` have the same length, so a size check alone misses it
    const second = className(await extract(hooks, app('tan')))

    expect(second).not.toBe(first)
    expect(sheets()).toEqual([sheet])
    expect(serve()).toContain(`.${second}{color:tan}`)
    expect(readFileSync(path, 'utf-8')).toBe(serve())
  })

  it('builds once more only in the environment whose served sheet went stale', async () => {
    const hooks = fakeApi(root)
    await plugin().setup(hooks.api)
    const node: Environment = { name: 'node', entry: {} }
    const builds = [web, node].map((environment) => {
      const config: { plugins?: RspackPlugin[] } = {}
      hooks.rspackConfigs[1](config, { environment })
      const fake = fakeCompiler()
      for (const rspackPlugin of config.plugins ?? [])
        rspackPlugin.apply(fake.compiler)
      fake.call('thisCompilation', fake.compilation)
      return { environment, ...fake }
    })
    await extract(hooks, app('red'))
    const path = join(root, 'df', 'devup-ui', sheets()[0])
    hooks.css({
      code: '',
      resourcePath: path,
      environment: web,
      addDependency: () => {},
      addMissingDependency: () => {},
    })
    hooks.css({
      code: '',
      resourcePath: path,
      environment: node,
      addDependency: () => {},
      addMissingDependency: () => {},
    })
    // Only the web build has seen the sheet change since it served it
    await extract(hooks, app('tan'))
    hooks.css({
      code: '',
      resourcePath: path,
      environment: node,
      addDependency: () => {},
      addMissingDependency: () => {},
    })

    const passes = builds.map((build) => {
      build.call('finishModules')
      build.call('processAssets')
      return build.call('needAdditionalPass')[0]
    })

    expect(passes).toEqual([true, false])
    const [webBuild, nodeBuild] = builds
    expect(webBuild.compilation.deleteAsset).toHaveBeenCalledWith('main.js')
    expect(webBuild.compiler.modifiedFiles).toEqual(new Set([path]))
    expect(nodeBuild.compilation.deleteAsset).not.toHaveBeenCalled()
    // A second pass is not asked for again in the same run
    webBuild.call('finishModules')
    expect(webBuild.call('needAdditionalPass')[0]).toBe(false)
    expect(readFileSync(path, 'utf-8')).toContain('color:tan')
  })
})
