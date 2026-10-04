import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { afterEach, expect, it, mock, spyOn } from 'bun:test'

import { DevupUI } from '../plugin'

const base = process.env.DEVUP_PLUGIN_TEST_TMP ?? realpathSync(tmpdir())
const roots: string[] = []
const closes: (() => void)[] = []
afterEach(() => {
  for (const close of closes.splice(0)) close()
  mock.restore()
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
  wasm.resetBuildState()
  wasm.setDebug(true)
})

function project() {
  mkdirSync(base, { recursive: true })
  const root = realpathSync(mkdtempSync(join(base, 'rsbuild-')))
  roots.push(root)
  return root
}
function file(root: string, name: string, contents: string) {
  const path = join(root, name)
  mkdirSync(dirname(path), { recursive: true })
  writeFileSync(path, contents)
  return path
}
const box = (color: string) =>
  `import { Box } from '@devup-ui/react'; export default <Box bg="${color}" />`

async function setup(
  root: string,
  options: Parameters<typeof DevupUI>[0] = {},
) {
  const transform = mock()
  const modify = mock()
  const configure = mock()
  const before = mock()
  await DevupUI(options).setup({
    context: { rootPath: root },
    transform,
    modifyRspackConfig: modify,
    modifyRsbuildConfig: configure,
    onBeforeBuild: before,
    onCloseBuild: mock((close) => closes.push(close)),
  })
  const config: { plugins?: { apply(compiler: unknown): void }[] } = {}
  const apply = (normalized: object) => {
    modify.mock.calls[0][0](config, {
      environment: { name: 'web', config: { output: { target: 'web' } } },
    })
    config.plugins?.at(-1)?.apply({
      options: normalized,
      hooks: { run: { tap: mock() }, thisCompilation: { tap: mock() } },
    })
  }
  return { transform, configure, before, apply }
}

it('writes only beneath the Rsbuild root and clears declarations when the theme is removed', async () => {
  const root = project()
  file(
    root,
    'devup.json',
    JSON.stringify({ theme: { colors: { default: { primary: 'red' } } } }),
  )
  await setup(root, { extractCss: false, distDir: 'generated' })
  expect(readFileSync(join(root, 'generated/theme.d.ts'), 'utf-8')).toContain(
    'primary',
  )
  file(root, 'devup.json', '{}')
  await setup(root, { extractCss: false, distDir: 'generated' })
  expect(
    readFileSync(join(root, 'generated/theme.d.ts'), 'utf-8'),
  ).not.toContain('primary')
  expect(
    readFileSync(join(root, 'generated/devup-ui/devup-ui.css'), 'utf-8'),
  ).not.toContain('--primary')
})

it.each(['{', JSON.stringify({ extends: ['./base.json'] })])(
  'fails initial setup for malformed or cyclic real config: %s',
  async (contents) => {
    const root = project()
    file(root, 'devup.json', contents)
    file(root, 'base.json', JSON.stringify({ extends: ['./devup.json'] }))
    await expect(setup(root)).rejects.toThrow(join(root, 'devup.json'))
  },
)

it('shares one conditional graph across canonical mapping, reach and reachable prewarm', async () => {
  const root = project()
  file(
    root,
    'src/first.tsx',
    `import Page from '../app/second'; import Widget from '../custom/widget'; import value from 'fixture'; export default [Page, Widget, value]`,
  )
  file(root, 'app/second.tsx', box('tan'))
  file(root, 'custom/widget.tsx', box('blue'))
  file(root, 'custom/dead.tsx', 'this is invalid and unreachable')
  file(
    root,
    'node_modules/fixture/package.json',
    JSON.stringify({
      name: 'fixture',
      exports: {
        browser: './browser.mjs',
        node: './node.cjs',
        default: './fallback.js',
      },
    }),
  )
  const browser = file(
    root,
    'node_modules/fixture/browser.mjs',
    `import { css } from '@devup-ui/react'; export default css({ color: 'green' })`,
  )
  file(root, 'node_modules/fixture/node.cjs', `throw new Error('wrong branch')`)
  const graph = spyOn(utils, 'buildStaticImportGraph')
  const canonical = spyOn(utils, 'buildCanonicalMap')
  const reach = spyOn(utils, 'computeFileReach')
  const reachable = spyOn(utils, 'computeReachableFiles')
  const extract = spyOn(wasm, 'codeExtract')
  const api = await setup(root, {
    atomHoist: 2,
    sourceDirs: 'custom',
    include: ['fixture'],
  })
  api.apply({
    entry: {
      first: { import: ['./src/first.tsx'] },
      second: { import: ['./app/second.tsx'] },
    },
    resolve: {
      conditionNames: ['browser'],
      byDependency: { esm: { conditionNames: ['import', '...'] } },
    },
  })
  const built = graph.mock.results[0]?.value
  expect(graph).toHaveBeenCalledTimes(1)
  expect(canonical.mock.calls[0]?.[0].graph).toBe(built)
  expect(reach.mock.calls[0]?.[0].graph).toBe(built)
  expect(reachable.mock.calls[0]?.[0].graph).toBe(built)
  expect(extract.mock.calls.map((call) => call[0])).toContain(
    browser.replaceAll('\\', '/'),
  )
  expect(extract.mock.calls.map((call) => call[0])).not.toContain(
    join(root, 'custom/dead.tsx').replaceAll('\\', '/'),
  )
  expect(wasm.getCss(null, false)).toContain('green')
})

it.each(['mts', 'cts', 'cjs', 'mjs'])(
  'transforms real modern .%s files',
  async (extension) => {
    const root = project()
    const path = file(
      root,
      `src/style.${extension}`,
      `import { css } from '@devup-ui/react'; export const style = css({ color: 'red' })`,
    )
    const api = await setup(root)
    api.apply({
      entry: { main: { import: [path] } },
      resolve: { conditionNames: ['browser'] },
    })
    const result = await api.transform.mock.calls[1][1]({
      code: readFileSync(path, 'utf-8'),
      resourcePath: path,
      addDependency: mock(),
      environment: { name: 'web' },
    })
    expect(result.code).not.toContain('css(')
    const sheet = result.code.match(/devup-ui(?:-\d+)?\.css/)?.[0]
    expect(
      wasm.getCss(utils.getFileNumByFilename(sheet ?? ''), true),
    ).toContain('red')
  },
)

it('extracts compiled MDX in the post-loader without prewarming raw markdown', async () => {
  const root = project()
  const path = file(
    root,
    'app/page.mdx',
    `# Markdown\n\nimport { Box } from '@devup-ui/react'\n\n<Box bg="red" />`,
  )
  const api = await setup(root)
  api.apply({
    entry: { main: { import: [path] } },
    resolve: { conditionNames: ['browser'] },
  })
  expect(wasm.getCss(null, false)).not.toContain('red')
  expect(api.transform.mock.calls[2][0]).toMatchObject({ order: 'post' })
  const result = await api.transform.mock.calls[2][1]({
    code: box('red'),
    resourcePath: path,
    addDependency: mock(),
    environment: { name: 'web' },
  })
  expect(result.code).not.toContain('<Box')
  const sheet = result.code.match(/devup-ui(?:-\d+)?\.css/)?.[0]
  expect(wasm.getCss(utils.getFileNumByFilename(sheet ?? ''), true)).toContain(
    'red',
  )
})

it('annotates located MDX failures and propagates non-MDX failures', async () => {
  const root = project()
  const api = await setup(root)
  spyOn(wasm, 'codeExtract').mockImplementation(() => {
    throw new Error(`${join(root, 'page.mdx')}:3:4: broken`)
  })
  await expect(
    api.transform.mock.calls[2][1]({
      code: '',
      resourcePath: join(root, 'page.mdx'),
      addDependency: mock(),
    }),
  ).rejects.toThrow('in compiled MDX')
  await expect(
    api.transform.mock.calls[1][1]({
      code: '',
      resourcePath: join(root, 'page.tsx'),
      addDependency: mock(),
    }),
  ).rejects.toThrow('broken')
})

it('fails prewarm with its real source filename and root', async () => {
  const root = project()
  const path = file(
    root,
    'src/style.ts',
    `import { css } from '@devup-ui/react'; export const style = css({ color: Math.random() })`,
  )
  const api = await setup(root)
  expect(() => api.apply({ entry: { main: { import: [path] } } })).toThrow(
    `prewarm failed at ${path}`,
  )
})

it('warns once for optional seed failure while preserving requested graph guarantees', async () => {
  const root = project()
  const api = await setup(root)
  spyOn(utils, 'collectNumberedFiles').mockImplementation(() => {
    throw new Error('seed failed')
  })
  const warn = spyOn(console, 'warn').mockImplementation(() => {})
  api.apply({ entry: {} })
  api.apply({ entry: {} })
  expect(warn).toHaveBeenCalledTimes(1)
  expect(warn.mock.calls[0]?.[1]).toMatchObject({ phase: 'seed', root })
})

it('fails rather than guessing when normalized entries are dynamic', async () => {
  const root = project()
  const api = await setup(root)
  expect(() => api.apply({ entry: () => ({}) })).toThrow(
    'dynamic Rspack entries',
  )
})

it('preserves user cache groups when shared splitting is composed', async () => {
  const root = project()
  const api = await setup(root, { atomHoist: 2 })
  const config: { tools?: { rspack?: (config: object) => void } } = {}
  api.configure.mock.calls[0][0](config)
  const vendor = { name: 'vendor' }
  const rspack = { optimization: { splitChunks: { cacheGroups: { vendor } } } }
  config.tools?.rspack?.(rspack)
  expect(rspack.optimization.splitChunks.cacheGroups).toMatchObject({
    vendor,
    devupUiShared: { enforce: true },
  })
})
