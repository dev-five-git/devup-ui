import { mkdirSync, utimesSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { MdxFreshnessError } from '../mdx-source-freshness'
import {
  affectedMdxSources,
  createMdxSourceManager,
} from '../mdx-source-generation'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it('changes CSS and compiles once when a real remark plugin reports changed data', async () => {
  // Given
  const f = sourceFixture(
    { 'app/page.mdx': styledMdx, 'app/data.json': '{"color":"purple"}' },
    {},
    true,
  )
  const first = await f.manager.prepare(f.signal)
  f.write('app/data.json', '{"color":"green"}')
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(f.counts()).toBe(2)
  expect(f.css(next)).toContain('color:green')
  expect(f.css(first)).toContain('color:purple')
})

it('reuses unchanged generation without compilation or extraction callbacks', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx }, {}, true)
  const first = await f.manager.prepare(f.signal)
  const calls = f.extractionCalls()
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(next).toBe(first)
  expect(f.counts()).toBe(1)
  expect(f.extractionCalls()).toBe(calls)
})

it('refreshes only affected modules when one of two route resources changes', async () => {
  // Given
  const f = sourceFixture(
    { 'app/a/page.mdx': styledMdx, 'app/b/page.mdx': styledMdx },
    {},
    true,
  )
  const first = await f.manager.prepare(f.signal)
  f.write('app/a/page.mdx', styledMdx.replace('red', 'blue'))
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(f.counts()).toBe(3)
  expect(next.compiled[join(f.root, 'app/b/page.mdx')]?.prepared).toBe(
    first.compiled[join(f.root, 'app/b/page.mdx')]?.prepared,
  )
  expect(f.css(next)).toContain('background:blue')
})

it.each(['directory', 'missing', 'build'] as const)(
  'reprepares when a reported %s input changes',
  async (kind) => {
    // Given
    const f = sourceFixture(
      { 'app/page.mdx': styledMdx, 'app/build.json': '{}' },
      {},
      true,
    )
    const directory = join(f.root, 'app/reported')
    mkdirSync(directory)
    utimesSync(
      directory,
      new Date(1_700_000_000_000),
      new Date(1_700_000_000_000),
    )
    const first = await f.manager.prepare(f.signal)
    const changed =
      kind === 'directory'
        ? f.write('app/reported/new.txt', 'new')
        : kind === 'missing'
          ? f.write('app/optional.json', '{}')
          : f.write('app/build.json', '{"changed":true}')
    if (kind === 'directory')
      utimesSync(
        directory,
        new Date(1_700_000_000_000),
        new Date(1_700_000_000_000),
      )
    // When
    const next = await f.manager.refresh({
      generation: first,
      signal: f.signal,
    })
    // Then
    expect(f.counts()).toBe(2)
    expect(next).not.toBe(first)
    expect(affectedMdxSources(first, changed)).toEqual([
      join(f.root, 'app/page.mdx'),
    ])
  },
)

it('rejects a dev result when a reported input is modified during its single compile', async () => {
  // Given
  const f = sourceFixture(
    { 'app/page.mdx': styledMdx, 'app/data.json': '{"color":"green"}' },
    {},
    true,
  )
  f.pluginOptions.before = async () => {
    f.write('app/data.json', '{"color":"blue"}', false)
  }
  // When / Then
  await expect(f.manager.prepare(f.signal)).rejects.toBeInstanceOf(
    MdxFreshnessError,
  )
  expect(f.counts()).toBe(1)
})

it('accepts a pregenerated production input but rejects changes before CSS finalization', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'app/data.json': '{"color":"green"}',
  })
  f.write('app/data.json', '{"color":"blue"}', false)
  const first = await f.manager.prepare(f.signal)
  f.manager.validateForCssFinalization(first)
  f.write('app/data.json', '{"color":"purple"}')
  // When / Then
  expect(() => f.manager.validateForCssFinalization(first)).toThrow(
    join(f.root, 'app/data.json'),
  )
})

it('reprepares an MDX importer when an ordinary resolver dependency changes outside discovery outputs', async () => {
  // Given
  const f = sourceFixture(
    {
      'app/page.mdx': `import { css } from '@devup-ui/react'\n\nimport { color } from '../df/value'\n\nexport const style = css({color})\n\n# Test`,
      'df/value.ts': "export const color = 'blue'",
    },
    {},
    true,
  )
  const first = await f.manager.prepare(f.signal)
  f.write('df/value.ts', "export const color = 'green'")
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(f.counts()).toBe(2)
  expect(first.watchInputs).toContain(join(f.root, 'df/value.ts'))
  expect(
    next.ordinaryInputs.some((input) => input.filename === 'df/value.ts'),
  ).toBe(true)
  expect(f.css(next)).toContain('color:green')
})

it('validates every reported input after a serialized private cache restart', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'app/data.json': '{"color":"blue"}',
    'app/build.json': '{}',
  })
  const first = await f.manager.prepare(f.signal)
  const saved = f.manager.restartCache(first)
  const restarted = createMdxSourceManager(f.binding, saved)
  await restarted.prepare(f.signal)
  expect(f.counts()).toBe(1)
  f.write('app/build.json', '{"updated":true}')
  // When
  const next = await createMdxSourceManager(f.binding, saved).prepare(f.signal)
  // Then
  expect(f.counts()).toBe(2)
  expect(f.css(next)).toContain('color:blue')
})

it('invalidates function-bearing pipeline identity instead of serializing it across restart', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  f.pluginOptions.before = async () => {}
  const first = await f.manager.prepare(f.signal)
  const saved = f.manager.restartCache(first)
  // When
  await createMdxSourceManager(f.binding, saved).prepare(f.signal)
  // Then
  expect(JSON.parse(saved)).toEqual([])
  expect(f.counts()).toBe(2)
})

it('does not replace a predecessor when cancellation retires an active compilation', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const first = await f.manager.prepare(f.signal)
  const started = Promise.withResolvers<void>()
  const release = Promise.withResolvers<void>()
  f.pluginOptions.before = async () => {
    started.resolve()
    await release.promise
  }
  f.write('app/page.mdx', styledMdx.replace('red', 'green'))
  const abort = new AbortController()
  const pending = f.manager.refresh({ generation: first, signal: abort.signal })
  await started.promise
  // When
  abort.abort(new Error('retired'))
  release.resolve()
  // Then
  await expect(pending).rejects.toThrow('retired')
  expect(first.sources[0]?.input.source).toContain('red')
})
