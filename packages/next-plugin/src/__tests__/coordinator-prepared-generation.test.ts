import { join } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import { createInput } from '../coordinator-ledger'
import type { PreparedSourceGeneration } from '../coordinator-options'
import { readCoordinatorState } from '../state'
import { createTestApp, removeTestApps } from './coordinator-app'
import {
  box,
  compiledSource,
  cssQuery,
  generation,
  preparedCore,
  prewarmed,
} from './coordinator-prepared-fixture'

afterEach(removeTestApps)

it('adopts a complete-empty refresh while retaining ordinary inputs', async () => {
  // Given a compiled source that the provider no longer reaches and an ordinary survivor.
  const app = createTestApp()
  app.write('src/page.mdx', '# raw')
  const ordinary = createInput(
    app.root,
    {
      filename: 'src/ordinary.tsx',
      resourcePath: join(app.root, 'src/ordinary.tsx'),
      code: box('blue'),
    },
    [],
  )
  const prepared = await preparedCore(app, {
    generation: generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('red') }),
    ]),
    ordinaryInputs: [ordinary],
    prepareReplay: async ({ generation: current }) =>
      current.sources.length === 0 ? current : generation([]),
  })

  // When a complete-empty prepared generation is reconciled.
  await prepared.core.reconcile()

  // Then old compiled inputs/styles are removed, while ordinary bytes/numbers survive.
  const snapshot = readCoordinatorState(prepared.options.stateFile ?? '', '')
  expect(snapshot?.inputs.map((input) => input.filename)).toEqual([
    'src/ordinary.tsx',
  ])
  expect(
    (await prepared.core.css({ ...cssQuery, fileNum: 1 })).css,
  ).not.toContain('background:red')
  expect((await prepared.core.css(cssQuery)).css).toContain('background:blue')
  expect(snapshot?.revision).toBe(8)
  prepared.core.close()
})

it('appends newly prepared source numbers without renumbering an earlier path', async () => {
  // Given a dev allocator that already numbered a later-sorted source.
  const app = createTestApp()
  app.write('src/z.mdx', '# existing')
  const existing = compiledSource(app, {
    filename: 'src/z.mdx',
    code: box('red'),
  })
  const staged: { generation?: PreparedSourceGeneration } = {}
  const prepared = await preparedCore(app, {
    generation: generation([existing]),
    prepareReplay: async ({ generation: current }) =>
      current.sources.length === 2 ? current : (staged.generation ?? current),
  })
  app.write('src/a.mdx', '# new')
  staged.generation = generation([
    compiledSource(app, { filename: 'src/a.mdx', code: box('green') }),
    existing,
  ])

  // When the newly reachable, earlier-sorted path enters prepared replay.
  await prepared.core.reconcile()

  // Then existing numbers remain and the new source appends to the allocator.
  const snapshot = readCoordinatorState(prepared.options.stateFile ?? '', '')
  expect(snapshot?.fileMap).toEqual({ 'src/z.mdx': 0, 'src/a.mdx': 1 })
  expect((await prepared.core.css(cssQuery)).css).toContain('background:red')
  expect((await prepared.core.css({ ...cssQuery, fileNum: 1 })).css).toContain(
    'background:green',
  )
  prepared.core.close()
})

it('snapshots immutable prepared evidence instead of retaining mutable provider record data', async () => {
  // Given provider records backed by mutable input arrays/records before Core adoption.
  const app = createTestApp()
  app.write('src/page.mdx', '# raw')
  const original = compiledSource(app, {
    filename: 'src/page.mdx',
    code: box('red'),
  })
  const input = { ...original.input }
  const sources = [{ ...original, input }]
  const observed: boolean[] = []
  const prepared = prewarmed(app, {
    generation: generation(sources),
    prepareReplay: async ({ generation: current }) => {
      const source = current.sources[0]
      observed.push(
        Object.isFrozen(current),
        Object.isFrozen(current.sources),
        Object.isFrozen(source?.input),
        Object.isFrozen(source?.input.stamps),
        Object.isFrozen(source?.evidence.fileFingerprints),
        Object.isFrozen(source?.evidence.contextFingerprints),
        Object.isFrozen(source?.evidence.missingDependencies),
      )
      return current
    },
  })
  const core = createCore(prepared.options, app.root)
  input.source = box('green')
  sources.splice(0)

  // When startup commits and CSS joins the immutable provider generation.
  await core.startup()
  const css = await core.css(cssQuery)

  // Then mutation of the original data cannot rewrite the adopted input/evidence.
  expect(css.css).toContain('background:red')
  expect(
    readCoordinatorState(prepared.options.stateFile ?? '', '')?.inputs[0]
      ?.source,
  ).toBe(box('red'))
  expect(observed).toEqual([true, true, true, true, true, true, true])
  core.close()
})

it('discards stale prewarmed cache entries outside the supplied complete inputs', async () => {
  // Given fresh prepared bytes and a stale/foreign initial output cache.
  const app = createTestApp()
  app.write('src/page.mdx', '# raw')
  const prepared = prewarmed(app, {
    generation: generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('green') }),
    ]),
  })
  const outputs = prepared.options.prewarmedOutputs
  if (outputs === undefined) throw new Error('fixture outputs missing')
  outputs.set('ghost.tsx', {
    source: box('red'),
    code: 'stale-cache-code',
    updatedBaseStyle: false,
  })
  outputs.set('src/page.mdx', {
    source: box('red'),
    code: 'stale-cache-code',
    updatedBaseStyle: false,
  })
  const core = createCore(prepared.options, app.root)
  await core.startup()

  // When the actual compiled source asks for its transform.
  const response = await core.extract({
    filename: 'src/page.mdx',
    resourcePath: join(app.root, 'src/page.mdx'),
    code: box('green'),
  })

  // Then only the complete prepared inputs are durable and stale cached output is never returned.
  expect(response.code).not.toBe('stale-cache-code')
  expect(response.code).toContain('className')
  expect(
    readCoordinatorState(prepared.options.stateFile ?? '', '')?.inputs.map(
      (input) => input.filename,
    ),
  ).toEqual(['src/page.mdx'])
  core.close()
})

it('serializes generation preparation with competing extract work', async () => {
  // Given a delayed provider and a second operation queued behind its candidate.
  const app = createTestApp()
  app.write('src/page.mdx', '# raw')
  const entered = Promise.withResolvers<void>()
  const release = Promise.withResolvers<PreparedSourceGeneration>()
  let calls = 0
  const prepared = await preparedCore(app, {
    generation: generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('red') }),
    ]),
    prepareReplay: ({ generation: current }) => {
      calls += 1
      if (calls === 1) {
        entered.resolve()
        return release.promise
      }
      return Promise.resolve(current)
    },
  })
  const css = prepared.core.css(cssQuery)
  await entered.promise
  const extracting = prepared.core.extract({
    filename: 'ordinary.tsx',
    resourcePath: join(app.root, 'ordinary.tsx'),
    code: box('blue'),
  })
  await Promise.resolve()
  expect(calls).toBe(1)
  expect(prepared.extractions).toEqual(['map:src/page.mdx'])

  // When the first queued generation finishes preparation.
  release.resolve(
    generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('green') }),
    ]),
  )

  // Then the competing request runs only after fresh replay is durable.
  expect((await css).css).toContain('background:green')
  expect((await extracting).cssFile).toEndWith('devup-ui-1.css')
  expect(calls).toBe(2)
  expect(
    readCoordinatorState(prepared.options.stateFile ?? '', '')?.revision,
  ).toBe(9)
  prepared.core.close()
})
