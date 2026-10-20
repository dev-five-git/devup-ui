import * as disk from 'node:fs/promises'
import { mkdtemp, readdir, readFile, rm, stat, utimes } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { getFileNumByFilename } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { afterEach, beforeEach, expect, it, mock, spyOn } from 'bun:test'
import type { Compiler, Configuration } from 'webpack'

import { bindCompilerScope, createWebpackGeneration } from '../build-scope'
import loader, { type DevupUILoaderOptions } from '../loader'
import { DevupUIWebpackPlugin } from '../plugin'

let root: string
const scopes: ReturnType<typeof bindCompilerScope>[] = []
function owner(prefix = '') {
  const compiler = { root: {} }
  const scope = bindCompilerScope(compiler)
  scope.setConfiguration(() => {
    wasm.setPrefix(prefix)
    wasm.registerShorthands({})
    wasm.registerTheme({})
  })
  scopes.push(scope)
  return { compiler, scope }
}
function options(): DevupUILoaderOptions {
  return {
    package: '@devup-ui/react',
    rootDir: root,
    cssDir: root,
    sheetFile: join(root, 'sheet.json'),
    classMapFile: join(root, 'class.json'),
    fileMapFile: join(root, 'file.json'),
    watch: true,
    singleCss: true,
  }
}
const source = (color = 'red') =>
  `import {Box} from '@devup-ui/react';export const view=<Box color="${color}"/>`
function extract(
  compiler: object,
  input: {
    source?: string
    file?: string
    options?: Partial<DevupUILoaderOptions>
  } = {},
) {
  const dependencies: string[] = []
  return new Promise<{ code: string | undefined; dependencies: string[] }>(
    (done, reject) => {
      Reflect.apply(
        loader,
        {
          _compiler: compiler,
          resourcePath: join(root, input.file ?? 'entry.tsx'),
          getOptions: () => input.options ?? options(),
          async: () => (error: Error | null, code?: string) =>
            error ? reject(error) : done({ code, dependencies }),
          addDependency: (path: string) => dependencies.push(path),
          addMissingDependency() {},
        },
        [Buffer.from(input.source ?? source())],
      )
    },
  )
}
beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), 'devup-loader-publication-'))
})
afterEach(async () => {
  mock.restore()
  for (const scope of scopes.splice(0)) scope.close()
  await rm(root, { recursive: true, force: true })
})

it.each([false, true])(
  'does not republish completed unchanged extraction (singleCss=%s)',
  async (singleCss) => {
    // Given: actual WASM extraction has published, then returns no incremental CSS.
    const { compiler, scope } = owner()
    const opts = { ...options(), singleCss }
    if (singleCss) await extract(compiler, { file: 'b.tsx', options: opts })
    await extract(compiler, { options: opts })
    const paths = (await readdir(root)).map((file) => join(root, file))
    const bytes = await Promise.all(
      paths.map((path) => readFile(path, 'utf-8')),
    )
    for (const path of paths) await utimes(path, 1, 1)
    const codeExtract = wasm.codeExtract
    spyOn(wasm, 'codeExtract').mockImplementation((...args) => {
      const output = codeExtract(...args)
      expect(output.cssFile).toBeDefined()
      expect(output.css).toBeUndefined()
      return output
    })
    const writes = spyOn(disk, 'writeFile')
    // When
    await extract(compiler, { options: opts })
    // Then: CSS is complete current target CSS, not a marker or an empty delta.
    expect(
      await Promise.all(paths.map((path) => readFile(path, 'utf-8'))),
    ).toEqual(bytes)
    expect(
      await Promise.all(paths.map(async (path) => (await stat(path)).mtimeMs)),
    ).toEqual(paths.map(() => 1000))
    expect(writes).not.toHaveBeenCalled()
    const cssPath = paths.find((path) => path.endsWith('.css'))
    if (cssPath)
      expect(await readFile(cssPath, 'utf-8')).toBe(
        scope.run(() => wasm.getCss(getFileNumByFilename(cssPath), true)),
      )
  },
)

it('publishes actual changed CSS and subscribes to supplied configuration inputs', async () => {
  // Given
  const { compiler } = owner()
  await extract(compiler)
  const configurationFiles = [
    join(root, 'devup.json'),
    join(root, 'tokens.css'),
  ]
  // When
  const result = await extract(compiler, {
    source: source('blue'),
    options: { ...options(), ...{ configurationFiles } },
  })
  // Then
  expect(await readFile(join(root, 'devup-ui.css'), 'utf-8')).toContain(
    'color:blue',
  )
  expect(result.dependencies).toEqual(
    expect.arrayContaining([
      ...configurationFiles,
      options().sheetFile,
      options().classMapFile,
      options().fileMapFile,
    ]),
  )
})

it('captures CSS within A/B/A owner operations and restores shared CSS handoff', async () => {
  // Given
  const a = owner('A-')
  const b = owner('B-')
  await extract(a.compiler)
  const first = await readFile(join(root, 'devup-ui.css'), 'utf-8')
  await extract(b.compiler, { source: source('blue') })
  expect(await readFile(join(root, 'devup-ui.css'), 'utf-8')).toContain('B-')
  // When
  await extract(a.compiler)
  // Then
  expect(await readFile(join(root, 'devup-ui.css'), 'utf-8')).toBe(first)
  expect(first).toContain('A-')
  const shared = createWebpackGeneration()
  const server = { root: {} }
  const client = { root: {} }
  const serverScope = bindCompilerScope(server, {
    owner: shared,
    complete: false,
  })
  const clientScope = bindCompilerScope(client, {
    owner: shared,
    complete: true,
  })
  scopes.push(serverScope, clientScope)
  serverScope.start()
  clientScope.start()
  await extract(server, { source: source('green') })
  const sharedCss = await readFile(join(root, 'devup-ui.css'), 'utf-8')
  serverScope.close()
  await extract(client, { source: source('green') })
  expect(await readFile(join(root, 'devup-ui.css'), 'utf-8')).toBe(sharedCss)
})

it('recreates deleted products and respects a fresh owner reusing the same paths', async () => {
  // Given
  const a = owner()
  await extract(a.compiler)
  const path = join(root, 'devup-ui.css')
  await rm(path)
  // When
  await extract(a.compiler)
  const recreated = await readFile(path, 'utf-8')
  a.scope.close()
  await extract(owner('fresh-').compiler, { source: source('purple') })
  // Then
  expect(recreated).toContain('color:red')
  expect(await readFile(path, 'utf-8')).toContain('fresh-')
  expect(await readFile(path, 'utf-8')).toContain('color:purple')
})

it('refreshes actual plugin configuration dependencies, responsive classes and shorthand CSS for unchanged source', async () => {
  // Given
  const installed = createRequire(
    join(process.cwd(), 'apps/landing/package.json'),
  )
  const bundled: { webpack(config: Configuration): Compiler } = installed(
    'next/dist/compiled/webpack/webpack',
  )
  const config = join(root, 'devup.json')
  const theme = (size: readonly (string | null)[], css: string) =>
    JSON.stringify({
      theme: { length: { default: { size } } },
      tailwind: { css },
    })
  await disk.writeFile(config, theme(['11px'], './a.css'))
  await disk.writeFile(join(root, 'a.css'), '@theme { --color-ink:red; }')
  await disk.writeFile(join(root, 'b.css'), '@theme { --color-ink:blue; }')
  const plugin = new DevupUIWebpackPlugin({
    watch: true,
    singleCss: true,
    shorthands: { insetX: ['left'] },
  })
  const compiler = bundled.webpack({
    context: root,
    mode: 'development',
    entry: {},
    plugins: [plugin],
  })
  try {
    const rule = compiler.options.module.rules[0]
    if (!rule || typeof rule !== 'object' || !Array.isArray(rule.use))
      throw new Error('source loader rule missing')
    const use = rule.use[0]
    if (!use || typeof use !== 'object') throw new Error('loader missing')
    const opts = use.options
    if (typeof opts !== 'object') throw new Error('loader options missing')
    const input = {
      source:
        'import {Box} from "@devup-ui/react";export const view=<Box w="$size" insetX={1}/>',
      options: opts,
    }
    await compiler.hooks.watchRun.promise(compiler)
    const before = await extract(compiler, input)
    const path = join(opts.cssDir, 'devup-ui.css')
    expect(await readFile(path, 'utf-8')).toContain('left:4px')
    expect(before.dependencies).toContain(join(root, 'a.css'))
    await disk.writeFile(config, theme(['11px', null, '33px'], './b.css'))
    await utimes(config, 2, 2)
    plugin.options.shorthands = { insetX: ['right'] }
    // When
    await compiler.hooks.watchRun.promise(compiler)
    const after = await extract(compiler, input)
    const css = await readFile(path, 'utf-8')
    // Then
    expect(after.dependencies).toContain(config)
    expect(after.dependencies).toContain(join(root, 'b.css'))
    expect(after.dependencies).not.toContain(join(root, 'a.css'))
    expect(after.code).not.toBe(before.code)
    expect(css).toContain('right:4px')
    expect(css).toContain('33px')
    expect(css.match(/width:var\(--size\)/g)).toHaveLength(2)
    expect(css).toContain('768px')
  } finally {
    await new Promise<void>((done, reject) =>
      compiler.close((error) => (error ? reject(error) : done())),
    )
  }
})
