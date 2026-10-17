import {
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rm,
  writeFile,
} from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { ConfigLoadError } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { afterEach, beforeEach, expect, it, spyOn } from 'bun:test'
import { createServer, type Plugin, type ViteDevServer } from 'vite'

import { DevupUI } from '../plugin'

type TestPlugin = Omit<Plugin, 'watchChange'> & {
  watchChange(id: string): Promise<void>
}

let root: string
let server: ViteDevServer | undefined
const temp = process.env.DEVUP_PLUGIN_TEST_TMP ?? tmpdir()

beforeEach(async () => {
  wasm.resetBuildState()
  root = await realpath(await mkdtemp(join(temp, 'vite-')))
  await writeFile(join(root, 'tsconfig.json'), '{}')
})

afterEach(async () => {
  await server?.close()
  server = undefined
  await rm(root, { recursive: true, force: true })
})

it.each(['ts', 'tsx', 'mts', 'cts', 'js', 'jsx', 'mjs', 'cjs'])(
  'extracts modern .%s modules under the actual Vite root',
  async (extension) => {
    await mkdir(join(root, 'src'))
    await writeFile(
      join(root, 'src', `main.${extension}`),
      "import {css} from '@devup-ui/react'; export const color = css({bg:'red'});",
    )
    server = await createServer({
      root,
      tsconfig: join(root, 'tsconfig.json'),
      configFile: false,
      optimizeDeps: { noDiscovery: true },
      server: { middlewareMode: true, hmr: false },
      plugins: [DevupUI({ singleCss: true })],
    })

    const result = await server.transformRequest(`/src/main.${extension}`)

    expect(result?.code).not.toContain('css({')
    expect(result?.code).toContain('devup-ui.css')
    expect(wasm.getCss(null, false)).toContain('red')
  },
)

it('clears declarations when a real theme file becomes empty', async () => {
  await writeFile(
    join(root, 'devup.json'),
    JSON.stringify({ theme: { colors: { default: { removed: 'red' } } } }),
  )
  const [plugin] = DevupUI() as unknown as [TestPlugin]
  server = await createServer({
    root,
    configFile: false,
    optimizeDeps: { noDiscovery: true },
    server: { middlewareMode: true, hmr: false, watch: null },
    plugins: [plugin],
  })
  await server.watcher.close()
  expect(await readFile(join(root, 'df/theme.d.ts'), 'utf-8')).toContain(
    'removed',
  )
  await writeFile(join(root, 'devup.json'), '{}')

  await plugin.watchChange(join(root, 'devup.json'))

  expect(await readFile(join(root, 'df/theme.d.ts'), 'utf-8')).not.toContain(
    'removed',
  )
})

it('preserves the prior theme on a malformed watched config', async () => {
  await writeFile(
    join(root, 'devup.json'),
    JSON.stringify({ theme: { colors: { default: { preserved: 'red' } } } }),
  )
  const [plugin] = DevupUI() as unknown as [TestPlugin]
  server = await createServer({
    root,
    configFile: false,
    optimizeDeps: { noDiscovery: true },
    server: { middlewareMode: true, hmr: false, watch: null },
    plugins: [plugin],
  })
  await server.watcher.close()
  const prior = await readFile(join(root, 'df/theme.d.ts'), 'utf-8')
  const priorCss = wasm.getCss(null, false)
  expect(server.watcher.getWatched()).toEqual({})
  await writeFile(join(root, 'devup.json'), '{')
  const diagnostic = spyOn(console, 'error').mockImplementation(() => {})
  try {
    console.error('[unrelated] config diagnostic from another server')
    await plugin.watchChange(join(root, 'devup.json'))
    expect(await readFile(join(root, 'df/theme.d.ts'), 'utf-8')).toBe(prior)
    const targeted = diagnostic.mock.calls.find(
      ([message]) =>
        message ===
        `[devup-ui] theme update failed at ${join(root, 'devup.json')}`,
    )
    expect(targeted?.[0]).toBe(
      `[devup-ui] theme update failed at ${join(root, 'devup.json')}`,
    )
    const error = targeted?.[1]
    expect(error).toBeInstanceOf(ConfigLoadError)
    if (!(error instanceof ConfigLoadError))
      throw new TypeError('Expected config load diagnostic')
    expect(error.file).toBe(join(root, 'devup.json'))
    expect(error.cause).toBeInstanceOf(SyntaxError)
    expect(wasm.getCss(null, false)).toBe(priorCss)
    expect(priorCss).toContain('--preserved')
  } finally {
    diagnostic.mockRestore()
  }
})

it('rejects a malformed setup config rather than clearing it silently', async () => {
  await writeFile(join(root, 'devup.json'), '{')

  await expect(
    createServer({ root, configFile: false, plugins: [DevupUI()] }),
  ).rejects.toThrow(join(root, 'devup.json'))
})

it.each(['client', 'ssr'])(
  'covers all roots and the included closure for environment %s',
  async (consumer) => {
    for (const dir of ['src', 'app', 'custom', 'node_modules/foo.bar/nested'])
      await mkdir(join(root, dir), { recursive: true })
    await writeFile(
      join(root, 'node_modules/foo.bar/package.json'),
      JSON.stringify({
        name: 'foo.bar',
        exports: {
          browser: './browser.mjs',
          node: './node.mjs',
          import: './import.mjs',
        },
      }),
    )
    await writeFile(
      join(root, 'node_modules/foo.bar/browser.mjs'),
      "export {color} from './nested/colors.mts'",
    )
    await writeFile(
      join(root, 'node_modules/foo.bar/nested/colors.mts'),
      "export const color = 'red'",
    )
    await writeFile(
      join(root, 'node_modules/foo.bar/import.mjs'),
      "export const color = 'green'",
    )
    await writeFile(
      join(root, 'node_modules/foo.bar/node.mjs'),
      "export const color = 'blue'",
    )
    await writeFile(
      join(root, 'src/style.mts'),
      "import {color} from 'foo.bar'; import {css} from '@devup-ui/react'; export const style = css({bg:color})",
    )
    await writeFile(join(root, 'app/main.ts'), "import '../src/style.mts'")
    await writeFile(join(root, 'custom/main.cts'), "import '../src/style.mts'")
    server = await createServer({
      root,
      configFile: false,
      tsconfig: join(root, 'tsconfig.json'),
      optimizeDeps: { noDiscovery: true },
      server: { middlewareMode: true, hmr: false },
      resolve: { conditions: ['browser', 'development|production'] },
      ssr: { resolve: { conditions: ['node', 'development|production'] } },
      build: {
        ssr: consumer === 'ssr',
        rollupOptions: {
          input: { app: 'app/main.ts', custom: 'custom/main.cts' },
        },
      },
      plugins: [
        DevupUI({
          sourceDirs: ['src', 'app', 'custom'],
          include: ['foo.bar'],
          atomHoist: 2,
          singleCss: true,
        }),
      ],
    })

    const result =
      consumer === 'ssr'
        ? await server.environments.ssr.transformRequest('/src/style.mts')
        : await server.transformRequest('/src/style.mts')

    expect(result?.code).not.toContain('css({')
    expect(wasm.getCss(null, false)).toContain(
      consumer === 'ssr' ? 'blue' : 'red',
    )
    expect(wasm.getCss(null, false)).not.toContain(
      consumer === 'ssr' ? 'red' : 'blue',
    )
  },
)

it('extracts MDX only after the installed MDX compiler has compiled raw markdown', async () => {
  const landing = createRequire(
    resolve(import.meta.dir, '../../../../apps/landing/package.json'),
  )
  const loader = landing.resolve('@mdx-js/loader')
  const mdx = await import(
    pathToFileURL(createRequire(loader).resolve('@mdx-js/mdx')).href
  )
  await writeFile(
    join(root, 'page.mdx'),
    'import {Box} from \'@devup-ui/react\'\n\n# Title\n\n<Box bg="red" />',
  )
  server = await createServer({
    root,
    configFile: false,
    optimizeDeps: { noDiscovery: true },
    resolve: {
      alias: { 'react/jsx-runtime': landing.resolve('react/jsx-runtime') },
    },
    server: { middlewareMode: true, hmr: false },
    plugins: [
      DevupUI({ singleCss: true }),
      {
        name: 'actual-mdx-compiler',
        enforce: 'pre',
        async transform(source, id) {
          if (!id.endsWith('.mdx')) return
          const output = await mdx.compile({ value: source, path: id })
          return { code: String(output), map: null }
        },
      },
    ],
  })

  const result = await server.transformRequest('/page.mdx')

  expect(result?.code).not.toContain('bg: "red"')
  expect(result?.code).toContain('devup-ui.css')
  expect(wasm.getCss(null, false)).toContain('red')
})
