import { utimesSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import { extractInput } from '../coordinator-engine'
import { createMdxSourceManager } from '../mdx-source-generation'
import { createWasm } from '../wasm'
import { paletteFixture, paletteMdx } from './mdx-resolution-fixture'
import { styledMdx } from './mdx-source-fixture'

it('replays real core CSS when only exports selects a new styling module', async () => {
  // Given
  const f = paletteFixture()
  const generation = await f.manager.prepare(f.signal)
  const context = f.binding.effectiveAppContext
  const engine = createWasm(context.root)
  generation.configureWasm(engine)
  const settings = {
    package: context.libPackage,
    cssDir: context.cssDir,
    singleCss: true,
    sourceMap: false,
    importAliases: {},
  }
  for (const input of generation.inputs) extractInput(engine, settings, input)
  const core = createCore(
    {
      wasm: engine,
      ...settings,
      projectRoot: f.root,
      coordinatorPortFile: join(f.root, 'port'),
      canonicalMap: {},
      watch: true,
      createEngine: () => createWasm(context.root),
      preparedSources: {
        initial: {
          ordinaryInputs: generation.ordinaryInputs,
          generation,
          revision: 1,
        },
        prepareReplay: (request) => f.manager.refresh(request),
      },
    },
    f.root,
  )
  try {
    await core.startup()
    expect(
      (await core.css({ importMainCss: false, wait: false })).css,
    ).toContain('color:red')
    // When
    f.select('blue')
    const after = await core.css({ importMainCss: false, wait: false })
    const cold = await f.manager.prepare(f.signal)
    // Then
    expect(generation.watchInputs).toContain(f.manifest)
    expect(f.css(cold)).toContain('color:blue')
    expect(after.css).toContain('color:blue')
    expect(after.css).not.toContain('color:red')
  } finally {
    core.close()
    await core.flush()
  }
})

it('compiles only the route whose extraction consulted changed exports', async () => {
  // Given
  const f = paletteFixture({
    'app/a/page.mdx': paletteMdx,
    'app/b/page.mdx': styledMdx,
  })
  const first = await f.manager.prepare(f.signal)
  f.select('blue')
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(f.counts()).toBe(3)
  expect(next.compiled[join(f.root, 'app/b/page.mdx')]?.prepared).toBe(
    first.compiled[join(f.root, 'app/b/page.mdx')]?.prepared,
  )
  expect(f.css(next)).toContain('color:blue')
})

it('reuses consulted metadata with unchanged bytes despite a timestamp change', async () => {
  // Given
  const f = paletteFixture()
  const first = await f.manager.prepare(f.signal)
  const calls = f.extractionCalls()
  utimesSync(f.manifest, new Date(), new Date())
  // When
  const next = await f.manager.refresh({
    generation: first,
    signal: f.signal,
    changedPaths: [f.manifest],
  })
  // Then
  expect(next).toBe(first)
  expect(f.counts()).toBe(1)
  expect(f.extractionCalls()).toBe(calls)
})

it('rejects consulted metadata changes before production CSS finalization', async () => {
  // Given
  const f = paletteFixture(undefined, false)
  const first = await f.manager.prepare(f.signal)
  f.select('blue')
  // When / Then
  expect(() => f.manager.validateForCssFinalization(first)).toThrow(f.manifest)
})

it('recompiles a stale serialized resolution proof after restart', async () => {
  // Given
  const f = paletteFixture()
  const first = await f.manager.prepare(f.signal)
  const cache = f.manager.restartCache(first)
  f.select('blue')
  // When
  const next = await createMdxSourceManager(f.binding, cache).prepare(f.signal)
  // Then
  expect(f.counts()).toBe(2)
  expect(f.css(next)).toContain('color:blue')
})
