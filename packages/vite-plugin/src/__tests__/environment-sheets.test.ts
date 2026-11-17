import { spawnSync } from 'node:child_process'
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, expect, it } from 'bun:test'
import type { Plugin, ViteBuilder } from 'vite'

import { createEnvironmentSheets } from '../environment-sheets'

type WriteBundle = (
  this: { environment?: { name: string } },
  options: { dir?: string },
  bundle: Record<string, unknown>,
) => void
type BuildApp = {
  order: string
  handler: (builder: ViteBuilder) => Promise<void>
}

const dirs: string[] = []
afterEach(() => {
  for (const dir of dirs.splice(0))
    rmSync(dir, { recursive: true, force: true })
})

function hooks(plugin: Plugin) {
  return {
    writeBundle: plugin.writeBundle as unknown as WriteBundle,
    buildApp: plugin.buildApp as unknown as BuildApp,
  }
}

function createBuilder(names: string[], built: string[] = []) {
  const order: string[] = []
  const environments = Object.fromEntries(
    names.map((name) => [name, { name, isBuilt: built.includes(name) }]),
  )
  const builder = {
    environments,
    build: async (environment: { name: string; isBuilt: boolean }) => {
      order.push(environment.name)
      environment.isBuilt = true
    },
  } as unknown as ViteBuilder
  return { builder, order }
}

it('is a build-only plugin shared by every environment, run after buildApp', () => {
  // Given
  const { plugin } = createEnvironmentSheets(() => '')
  // Then
  expect(plugin.name).toBe('devup-ui:complete-environment-sheets')
  expect(plugin.apply).toBe('build')
  expect(plugin.sharedDuringBuild).toBe(true)
  expect(hooks(plugin).buildApp.order).toBe('post')
})

it('builds every environment once when no sheet changed', async () => {
  // Given
  const sheets = createEnvironmentSheets(() => 'a{}')
  const { builder, order } = createBuilder(['client', 'ssr'])
  // When
  await hooks(sheets.plugin).buildApp.handler(builder)
  // Then
  expect(order).toEqual(['client', 'ssr'])
})

it('leaves environments already built by an earlier buildApp hook', async () => {
  // Given
  const sheets = createEnvironmentSheets(() => 'a{}')
  const { builder, order } = createBuilder(['client', 'ssr'], ['client'])
  // When
  await hooks(sheets.plugin).buildApp.handler(builder)
  // Then
  expect(order).toEqual([])
})

it('builds again only the environments whose sheets changed since', async () => {
  // Given
  let css = 'a{}'
  const sheets = createEnvironmentSheets(() => css)
  const { builder, order } = createBuilder(['client', 'ssr'])
  const build = builder.build.bind(builder)
  builder.build = async (environment) => {
    await build(environment)
    // The server extracts a module the client never imports
    if (environment.name === 'ssr') css = 'a{}b{}'
    sheets.emit(environment.name).set(null, css)
  }
  // When
  await hooks(sheets.plugin).buildApp.handler(builder)
  // Then
  expect(order).toEqual(['client', 'ssr', 'client'])
})

it('removes the output an environment wrote before building it again', async () => {
  // Given
  const dir = mkdtempSync(join(tmpdir(), 'devup-environment-sheets-'))
  dirs.push(dir)
  mkdirSync(join(dir, 'assets'))
  writeFileSync(join(dir, 'assets', 'devup-ui-a1.css'), 'a{}')
  writeFileSync(join(dir, 'index.html'), '')
  let css = 'a{}'
  const sheets = createEnvironmentSheets(() => css)
  const { writeBundle, buildApp } = hooks(sheets.plugin)
  writeBundle.call(
    { environment: { name: 'client' } },
    { dir },
    { 'assets/devup-ui-a1.css': {}, 'index.html': {} },
  )
  sheets.emit('client').set(1, css)
  css = 'a{}b{}'
  const { builder } = createBuilder(['client', 'ssr'], ['client', 'ssr'])
  const exists: boolean[] = []
  const build = builder.build.bind(builder)
  builder.build = async (environment) => {
    exists.push(existsSync(join(dir, 'assets', 'devup-ui-a1.css')))
    exists.push(existsSync(join(dir, 'index.html')))
    await build(environment)
  }
  // When
  await buildApp.handler(builder)
  // Then
  expect(exists).toEqual([false, false])
})

it('builds again an environment whose output was not written', async () => {
  // Given
  let css = 'a{}'
  const sheets = createEnvironmentSheets(() => css)
  const { writeBundle, buildApp } = hooks(sheets.plugin)
  writeBundle.call({ environment: { name: 'client' } }, {}, { 'a.css': {} })
  writeBundle.call({}, { dir: 'dist' }, { 'a.css': {} })
  sheets.emit('client').set(1, css)
  css = 'b{}'
  const { builder, order } = createBuilder(['client'], ['client'])
  // When
  await buildApp.handler(builder)
  // Then
  expect(order).toEqual(['client'])
})

function build(order: string, mode: string) {
  // Vite cannot resolve modules under a Windows 8.3 short path
  const root = join(
    realpathSync.native(tmpdir()),
    `devup-environment-sheets-${order.replace(',', '-')}-${mode}`,
  )
  dirs.push(root)
  const result = spawnSync(
    process.execPath,
    [join(import.meta.dir, 'environment-sheets-build.ts'), root, order, mode],
    { encoding: 'utf8' },
  )
  expect(result.stderr).toBe('')
  return JSON.parse(result.stdout) as { client: string[]; server: string[] }
}

it.each(['single', 'split'])(
  'gives every environment the styles of its sheets whatever the build order (%s)',
  (mode) => {
    // When
    const clientFirst = build('client,ssr', mode)
    const ssrFirst = build('ssr,client', mode)
    // Then
    if (mode === 'single') {
      expect(clientFirst.client.join()).toContain('color:green')
      expect(clientFirst).toEqual(ssrFirst)
    }
    for (const output of [clientFirst, ssrFirst]) {
      expect(output.client.join()).toContain('color:red')
      expect(output.client.join()).toContain('color:blue')
      expect(output.server.join()).toContain('color:green')
      expect(output.server.join()).toContain('color:blue')
    }
  },
  120_000,
)
