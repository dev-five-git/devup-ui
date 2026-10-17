import { randomUUID } from 'node:crypto'
import { readFileSync, renameSync } from 'node:fs'
import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

import { startCoordinator } from '../coordinator'
import { extractInput } from '../coordinator-engine'
import type { PreparedSourceGeneration } from '../coordinator-options'
import { createMdxSourceManager } from '../mdx-source-generation'
import { createWasm } from '../wasm'
import { connect } from './coordinator-app'
import { paletteFixture } from './mdx-resolution-fixture'

function packagedFixture(development = true) {
  const f = paletteFixture(undefined, development)
  f.write('tsconfig.json', '{"extends":"preset"}')
  for (const color of ['red', 'blue'] as const) {
    f.write(
      `node_modules/preset/${color}.json`,
      JSON.stringify({
        compilerOptions: { paths: { palette: [`./${color}.js`] } },
      }),
    )
    f.write(
      `node_modules/preset/${color}.js`,
      `export const color = ${JSON.stringify(color)}`,
    )
  }
  const manifest = join(f.root, 'node_modules/preset/package.json')
  function select(color: 'red' | 'blue') {
    const replacement = f.write(
      'node_modules/preset/package.next.json',
      JSON.stringify({ exports: `./${color}.json` }),
    )
    renameSync(replacement, manifest)
  }
  select('red')
  return { ...f, manifest, select }
}

function fileProof(generation: PreparedSourceGeneration, path: string) {
  const proof = generation.resolutionInputs?.find(
    (input) => input.path === path && input.kind === 'file',
  )
  if (proof === undefined) throw new TypeError(`Missing file proof: ${path}`)
  return proof
}

it('replaces packaged config selection through a coordinator watch before HTTP CSS', async () => {
  // Given
  const f = packagedFixture()
  const first = await f.manager.prepare(f.signal)
  const pid = process.pid
  const red = join(f.root, 'node_modules/preset/red.json')
  const blue = join(f.root, 'node_modules/preset/blue.json')
  const originalProof = first.resolutionInputs
  const originalBytes = JSON.stringify(originalProof)
  const manifestFingerprint = fileProof(first, f.manifest).fingerprint
  fileProof(first, red)
  expect(first.watchInputs).toContain(f.manifest)
  expect(first.watchInputs).toContain(red)
  const unchanged: readonly Readonly<{ path: string; bytes: string }>[] = [
    'tsconfig.json',
    'app/page.mdx',
    'provider.tsx',
    'node_modules/preset/red.json',
    'node_modules/preset/blue.json',
    'node_modules/preset/red.js',
    'node_modules/preset/blue.js',
  ].map((path) => ({
    path: join(f.root, path),
    bytes: readFileSync(join(f.root, path), 'utf-8'),
  }))
  const context = f.binding.effectiveAppContext
  const wasm = createWasm(f.root)
  first.configureWasm(wasm)
  const settings = {
    package: context.libPackage,
    cssDir: context.cssDir,
    singleCss: true,
    sourceMap: false,
    importAliases: {},
  }
  for (const input of first.inputs) extractInput(wasm, settings, input)
  const identity = { project: f.root, token: randomUUID() }
  const portFile = join(f.root, 'coordinator.port')
  const event = Promise.withResolvers<{
    readonly generation: PreparedSourceGeneration
    readonly changedPaths: readonly string[]
  }>()
  let observedConfigurer = first.configureWasm
  const handle = startCoordinator({
    wasm,
    ...settings,
    projectRoot: f.root,
    coordinatorPortFile: portFile,
    identity,
    canonicalMap: {},
    watch: true,
    sourceRoots: [join(f.root, 'app')],
    createEngine: () => createWasm(f.root),
    preparedSources: {
      initial: {
        generation: first,
        ordinaryInputs: first.ordinaryInputs,
        revision: 1,
      },
      async prepareReplay(request) {
        try {
          observedConfigurer = request.generation.configureWasm
          const generation = await f.manager.refresh(request)
          const changedPaths = request.changedPaths ?? []
          if (
            changedPaths.some(
              (path) => path === f.manifest || path === dirname(f.manifest),
            )
          )
            event.resolve({ generation, changedPaths })
          return generation
        } catch (cause) {
          event.reject(cause)
          throw cause
        }
      },
    },
  })
  let timer: ReturnType<typeof setTimeout> | undefined
  try {
    await Promise.all([handle.ready, handle.prepared])
    const client = connect(portFile, identity)
    const initial = await client.get('/css')
    expect(initial.status).toBe(200)
    expect(initial.body).toContain('color:red')
    expect(initial.body).not.toContain('color:blue')
    expect(initial.body).not.toContain('color:var(')
    expect(f.counts()).toBe(1)
    timer = setTimeout(
      () => event.reject(new TypeError('Packaged watch did not refresh')),
      4000,
    )
    // When
    f.select('blue')
    // Then: the watcher must finish preparation before HTTP can reconcile.
    const watched = await event.promise
    clearTimeout(timer)
    expect(
      watched.changedPaths.some(
        (path) => path === f.manifest || path === dirname(f.manifest),
      ),
    ).toBe(true)
    expect(watched.generation.configureWasm).not.toBe(first.configureWasm)
    expect(fileProof(watched.generation, f.manifest).fingerprint).not.toBe(
      manifestFingerprint,
    )
    fileProof(watched.generation, blue)
    expect(watched.generation.watchInputs).toContain(f.manifest)
    expect(watched.generation.watchInputs).toContain(blue)
    const hot = await client.get('/css')
    expect(hot.status).toBe(200)
    expect(hot.body).toContain('color:blue')
    expect(hot.body).not.toContain('color:red')
    expect(hot.body).not.toContain('color:var(')
    expect(observedConfigurer).toBe(watched.generation.configureWasm)
    expect(process.pid).toBe(pid)
    expect(connect(portFile, identity).info).toEqual(client.info)
    expect(f.counts()).toBe(2)
    expect(first.resolutionInputs).toBe(originalProof)
    expect(JSON.stringify(first.resolutionInputs)).toBe(originalBytes)
    for (const input of unchanged)
      expect(readFileSync(input.path, 'utf-8')).toBe(input.bytes)
  } finally {
    clearTimeout(timer)
    await handle.drain()
  }
  const cold = await createMdxSourceManager(f.binding).prepare(f.signal)
  const coldCss = f.css(cold)
  expect(coldCss).toContain('color:blue')
  expect(coldCss).not.toContain('color:red')
  fileProof(cold, f.manifest)
  fileProof(cold, blue)
  expect(process.pid).toBe(pid)
})

it('rejects a selecting manifest change at production CSS finalization', async () => {
  // Given
  const f = packagedFixture(false)
  const first = await f.manager.prepare(f.signal)
  fileProof(first, f.manifest)
  expect(f.css(first)).toContain('color:red')
  f.select('blue')
  // When / Then
  expect(() => f.manager.validateForCssFinalization(first)).toThrow(f.manifest)
})

it('retains an independent root generation when another packaged manifest changes', async () => {
  // Given
  const changed = packagedFixture()
  const control = packagedFixture()
  const changedFirst = await changed.manager.prepare(changed.signal)
  const first = await control.manager.prepare(control.signal)
  const filename = join(control.root, 'app/page.mdx')
  const prepared = first.compiled[filename]?.prepared
  if (prepared === undefined)
    throw new TypeError('Control MDX was not prepared')
  expect(control.counts()).toBe(1)
  const proof = JSON.stringify(first.resolutionInputs)
  // When
  changed.select('blue')
  // Then
  const changedNext = await changed.manager.refresh({
    generation: changedFirst,
    signal: changed.signal,
    changedPaths: [changed.manifest],
  })
  expect(changed.css(changedNext)).toContain('color:blue')
  const next = await control.manager.refresh({
    generation: first,
    signal: control.signal,
    changedPaths: [changed.manifest],
  })
  expect(next).toBe(first)
  expect(next.compiled[filename]?.prepared).toBe(prepared)
  expect(control.counts()).toBe(1)
  expect(JSON.stringify(next.resolutionInputs)).toBe(proof)
  const css = control.css(next)
  expect(css).toContain('color:red')
  expect(css).not.toContain('color:blue')
})
