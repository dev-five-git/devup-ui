import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import * as pluginUtils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { afterEach, beforeEach, expect, it, spyOn } from 'bun:test'
import type { Compiler, Configuration } from 'webpack'

import { DevupUIWebpackPlugin } from '../plugin'
const landing = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const bundled: { webpack(config: Configuration): Compiler } = landing(
  'next/dist/compiled/webpack/webpack',
)
function webpack(config: Configuration) {
  return bundled.webpack({
    ...config,
    optimization: { ...config.optimization, minimize: false },
  })
}

let root: string
let compiler: Compiler | undefined

beforeEach(async () => {
  wasm.resetBuildState()
  root = await mkdtemp(
    join(process.env.DEVUP_PLUGIN_TEST_TMP ?? tmpdir(), 'webpack-'),
  )
  await mkdir(join(root, 'src'))
})

afterEach(async () => {
  if (compiler)
    await new Promise<void>((done, reject) =>
      compiler?.close((error) => (error ? reject(error) : done())),
    )
  compiler = undefined
  await rm(root, { recursive: true, force: true })
})

it.each(['ts', 'tsx', 'mts', 'cts', 'js', 'jsx', 'mjs', 'cjs'])(
  'prewarms modern .%s files relative to compiler.context',
  async (extension) => {
    await writeFile(
      join(root, 'src', `main.${extension}`),
      "import {css} from '@devup-ui/react'; export const style = css({bg:'red'});",
    )
    const plugin = new DevupUIWebpackPlugin({ singleCss: true })

    compiler = webpack({
      context: root,
      mode: 'production',
      entry: `./src/main.${extension}`,
      plugins: [plugin],
    })

    expect(plugin.options.distDir).toBe(join(root, 'df'))
    expect(wasm.getCss(null, false)).toContain('red')
    const rules = compiler.options.module.rules
    expect(rules).toContainEqual(
      expect.objectContaining({
        test: pluginUtils.SOURCE_FILE_RE,
        enforce: 'pre',
      }),
    )
    expect(rules).toContainEqual(
      expect.objectContaining({ enforce: 'post', test: /\.mdx$/i }),
    )
  },
)

it('overwrites empty declarations using the actual compiler context', async () => {
  await writeFile(
    join(root, 'devup.json'),
    JSON.stringify({ theme: { colors: { default: { removed: 'red' } } } }),
  )
  const plugin = new DevupUIWebpackPlugin()
  compiler = webpack({ context: root, plugins: [plugin] })
  expect(await readFile(join(root, 'df/theme.d.ts'), 'utf-8')).toContain(
    'removed',
  )
  await writeFile(join(root, 'devup.json'), '{}')

  plugin.writeDataFiles()

  expect(await readFile(join(root, 'df/theme.d.ts'), 'utf-8')).not.toContain(
    'removed',
  )
})

it.each(['web', 'node'])(
  'reuses one graph covering all roots and the included closure for target %s',
  async (target) => {
    await mkdir(join(root, 'app'))
    await mkdir(join(root, 'custom'))
    const library = join(root, 'node_modules', 'foo.bar')
    await mkdir(join(library, 'nested'), { recursive: true })
    await writeFile(
      join(library, 'package.json'),
      JSON.stringify({
        name: 'foo.bar',
        exports: {
          browser: './browser.mjs',
          node: './node.mjs',
          default: './fallback.js',
        },
      }),
    )
    await writeFile(
      join(library, 'browser.mjs'),
      "export {color} from './nested/colors.mts'",
    )
    await writeFile(
      join(library, 'nested/colors.mts'),
      "export const color = 'red'",
    )
    await writeFile(join(library, 'node.mjs'), "export const color = 'blue'")
    await writeFile(
      join(library, 'fallback.js'),
      "export const color = 'green'",
    )
    await writeFile(
      join(root, 'src/main.mts'),
      "import {color} from 'foo.bar'; import {css} from '@devup-ui/react'; export const style = css({bg:color})",
    )
    await writeFile(join(root, 'app/page.tsx'), "import '../src/main.mts'")
    await writeFile(join(root, 'custom/entry.cts'), "import '../src/main.mts'")
    const originalGraph = pluginUtils.buildStaticImportGraph
    const captured: { graph?: pluginUtils.StaticImportGraph } = {}
    const graph = spyOn(
      pluginUtils,
      'buildStaticImportGraph',
    ).mockImplementation((...args) => {
      captured.graph = originalGraph(...args)
      return captured.graph
    })
    const canonical = spyOn(pluginUtils, 'buildCanonicalMap')
    const reach = spyOn(pluginUtils, 'computeFileReach')
    const prewarm = spyOn(pluginUtils, 'computeReachableFiles')
    try {
      compiler = webpack({
        context: root,
        mode: 'production',
        target,
        entry: { app: './app/page.tsx', custom: './custom/entry.cts' },
        plugins: [
          new DevupUIWebpackPlugin({
            sourceDirs: ['src', 'app', 'custom'],
            include: ['foo.bar'],
            atomHoist: 2,
            singleCss: true,
          }),
        ],
      })

      expect(graph).toHaveBeenCalledTimes(1)
      const shared = captured.graph
      expect(shared?.files).toContain(
        join(library, target === 'web' ? 'nested/colors.mts' : 'node.mjs'),
      )
      expect(shared?.files).not.toContain(
        join(library, target === 'web' ? 'node.mjs' : 'nested/colors.mts'),
      )
      expect(canonical.mock.calls[0]?.[0].graph).toBe(shared)
      expect(reach.mock.calls[0]?.[0].graph).toBe(shared)
      expect(prewarm.mock.calls[0]?.[0].graph).toBe(shared)
      expect(wasm.getCss(null, false)).toContain(
        target === 'web' ? 'red' : 'blue',
      )
    } finally {
      graph.mockRestore()
      canonical.mockRestore()
      reach.mockRestore()
      prewarm.mockRestore()
    }
  },
)

it('rejects malformed config with its root and original cause', async () => {
  await writeFile(join(root, 'devup.json'), '{')

  expect(() =>
    webpack({ context: root, plugins: [new DevupUIWebpackPlugin()] }),
  ).toThrow(join(root, 'devup.json'))
})

it('does not parse raw MDX during prewarm', async () => {
  await writeFile(
    join(root, 'src/page.mdx'),
    'import {Box} from \'@devup-ui/react\'\n\n# Heading\n\n<Box bg="red" />',
  )
  const extract = spyOn(wasm, 'codeExtract')
  try {
    compiler = webpack({
      context: root,
      entry: './src/page.mdx',
      plugins: [new DevupUIWebpackPlugin()],
    })

    expect(extract).not.toHaveBeenCalled()
  } finally {
    extract.mockRestore()
  }
})

it.each([
  ['giant Next pitch', `next-app-loader?page=${'segment/'.repeat(5000)}!`],
  ['virtual scheme', 'virtual:next-entry'],
  ['virtual ID', '\0next-entry'],
  ['empty resource', 'loader?pitch=true!?resourceQueryOnly'],
])('skips virtual or pitch-only entry %s', async (_name, entry) => {
  const reachable = spyOn(pluginUtils, 'computeReachableFiles')
  try {
    compiler = webpack({
      context: root,
      mode: 'production',
      entry,
      plugins: [new DevupUIWebpackPlugin()],
    })
    expect(reachable.mock.calls[0]?.[0].entries).toEqual([])
  } finally {
    reachable.mockRestore()
  }
})

it.each([
  './src/main.ts',
  './src/main.ts?resource=true',
  'first-loader?value=one!second-loader?value=two!./src/main.ts?resource=true',
])('prewarms the real resource of entry %s', async (entry) => {
  await writeFile(
    join(root, 'src/main.ts'),
    "import {css} from '@devup-ui/react'; export const style = css({bg:'red'})",
  )
  await writeFile(
    join(root, 'src/unreachable.ts'),
    "import {css} from '@devup-ui/react'; export const style = css({bg:'blue'})",
  )
  compiler = webpack({
    context: root,
    mode: 'production',
    entry,
    plugins: [new DevupUIWebpackPlugin({ singleCss: true })],
  })
  expect(wasm.getCss(null, false)).toContain('red')
  expect(wasm.getCss(null, false)).not.toContain('blue')
})

it('propagates invalid real entry resource I/O instead of skipping it', () => {
  expect(() =>
    webpack({
      context: root,
      entry: 'loader!./src/\0.ts?resource=true',
      plugins: [new DevupUIWebpackPlugin()],
    }),
  ).toThrow()
})
