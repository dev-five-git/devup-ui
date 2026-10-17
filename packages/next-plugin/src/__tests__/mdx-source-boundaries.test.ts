import { mkdirSync, rmSync } from 'node:fs'
import { join } from 'node:path'

import { createModuleResolver } from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'

import { createMdxDeadline } from '../mdx-prepare'
import { withMdxSourceControl } from '../mdx-source-control'
import {
  captureMdxFreshness,
  changedMdxInput,
  createMdxTimestampAccuracy,
  fingerprintMdxInput,
  MdxFreshnessError,
} from '../mdx-source-freshness'
import { createMdxSourceManager } from '../mdx-source-generation'
import { runMdxSourcePreparation } from '../mdx-source-run'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it('rejects expiry when selection has not yet supplied a pipeline', async () => {
  // Given
  const signal = new AbortController().signal
  // When / Then
  await expect(
    withMdxSourceControl(
      { filename: '/project/page.mdx', signal, deadline: createMdxDeadline(1) },
      async () => new Promise<never>(() => {}),
    ),
  ).rejects.toBeInstanceOf(MdxFreshnessError)
})

it('propagates cancellation when async binding ignores its signal', async () => {
  // Given
  const controller = new AbortController()
  const started = Promise.withResolvers<void>()
  const pending = withMdxSourceControl(
    { filename: '/project/page.mdx', signal: controller.signal },
    async () => {
      started.resolve()
      return new Promise<never>(() => {})
    },
  )
  await started.promise
  // When
  controller.abort(new Error('cancel binding'))
  // Then
  await expect(pending).rejects.toThrow('cancel binding')
})

it('rejects an expired shared budget before touching the configured compiler', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  // When / Then
  await expect(
    runMdxSourcePreparation(f.binding, {
      signal: f.signal,
      candidates: new Map(),
      dirty: new Set(),
      deadline: createMdxDeadline(-1),
    }),
  ).rejects.toBeInstanceOf(MdxFreshnessError)
  expect(f.counts()).toBe(0)
})

it('rejects an excluded MDX import at its actual extraction importer', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': `import { css } from '@devup-ui/react'\n\nimport { color } from '../df/value.mdx'\n\nexport const style = css({color})\n\n# Test`,
    'df/value.mdx': `export const color = 'blue'`,
  })
  // When / Then
  await expect(f.manager.prepare(f.signal)).rejects.toThrow('df/value.mdx')
})

it('returns a pending expectation rather than reading a native-only resolver dependency', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': `import { css } from '@devup-ui/react'\n\nimport { color } from '../df/value'\n\nexport const style = css({color})\n\n# Test`,
    'df/value.ts': 'this source is invalid until upstream loader transforms it',
  })
  const filename = join(f.root, 'df/value.ts')
  const manager = createMdxSourceManager({
    ...f.binding,
    ordinaryEligibility: (path) =>
      path === filename
        ? {
            kind: 'native-required',
            expectation: { filename, reason: 'native-transform', proof: {} },
          }
        : { kind: 'disk-first' },
  })
  // When
  const generation = await manager.prepare(f.signal)
  // Then
  expect(generation.pendingOrdinary.map((item) => item.filename)).toEqual([
    filename,
  ])
  const resolver = createModuleResolver({ cwd: f.root, ...generation.resolver })
  expect(() => resolver('../df/value', 'app/page.mdx')).toThrow(
    'native upstream bytes',
  )
})

it('reports the actual reporting loader when it supplies a relative dependency', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  f.write(
    'reporting-raw.cjs',
    `module.exports = function(source) { this.addBuildDependency('relative.json'); return source }`,
  )
  // When / Then
  await expect(f.manager.prepare(f.signal)).rejects.toThrow(
    `relative.json reported by ${join(f.root, 'reporting-raw.cjs')}`,
  )
})

it('retains the native dependency alias identity while recording its reported file', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'app/page.mdx.alias': '{}',
  })
  f.write(
    'reporting-raw.cjs',
    `module.exports = function(source) { if (this.dependency !== this.addDependency) throw new Error('native alias identity changed'); this.dependency(this.resourcePath + '.alias'); require(this.getOptions().plugin).setContext(this); return source }`,
  )
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  expect(generation.watchInputs).toContain(join(f.root, 'app/page.mdx.alias'))
})

it('allows corrected retry after a required compiler failure without mutating old sources', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const first = await f.manager.prepare(f.signal)
  f.write('app/page.mdx', '<Box')
  await expect(
    f.manager.refresh({ generation: first, signal: f.signal }),
  ).rejects.toThrow(join(f.root, 'app/page.mdx'))
  f.write('app/page.mdx', styledMdx.replace('red', 'green'))
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(f.css(next)).toContain('background:green')
  expect(first.sources[0]?.input.source).toContain('red')
})

it('watches and refreshes a real reported file outside the project root', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx }, {}, true)
  const path = f.write(
    '../outside-' + f.root.split(/[\\/]/).at(-1) + '.json',
    '{"color":"blue"}',
  )
  try {
    f.pluginOptions.data = path
    const first = await f.manager.prepare(f.signal)
    f.write(path, '{"color":"green"}')
    // When
    const next = await f.manager.refresh({
      generation: first,
      signal: f.signal,
    })
    // Then
    expect(first.watchInputs).toContain(path)
    expect(f.counts()).toBe(2)
    expect(f.css(next)).toContain('color:green')
  } finally {
    rmSync(path)
  }
})

it('treats missing and unreadable reported inputs as changes rather than reusing cache', () => {
  // Given
  const f = sourceFixture({ 'data.json': '{}' })
  const path = join(f.root, 'data.json')
  const input = fingerprintMdxInput(path, {
    kind: 'build',
    path,
    loader: 'reporter',
  })
  rmSync(path)
  // When / Then
  expect(changedMdxInput(path, [input])).toBe(input)
  expect(() =>
    fingerprintMdxInput(path, { kind: 'context', path, loader: 'reporter' }),
  ).toThrow(path)
  expect(() =>
    fingerprintMdxInput(path, {
      kind: 'file',
      path: 'relative',
      loader: 'reporter',
    }),
  ).toThrow('not absolute')
  expect(() =>
    fingerprintMdxInput(path, {
      kind: 'file',
      path: join(f.root, '\u0000invalid'),
      loader: 'reporter',
    }),
  ).toThrow('reporter')
})

it('refuses to stamp a reported missing path that appeared during compile', () => {
  // Given
  const f = sourceFixture({ 'present.json': '{}' })
  const filename = join(f.root, 'page.mdx')
  // When / Then
  expect(() =>
    captureMdxFreshness(
      {
        filename,
        source: '',
        map: undefined,
        dependencies: [],
        buildDependencies: [],
        contextDependencies: [],
        missingDependencies: [join(f.root, 'present.json')],
        dependencyReports: [],
      },
      {
        development: false,
        startedAt: 0,
        accuracy: createMdxTimestampAccuracy(),
      },
    ),
  ).toThrow('appeared during compilation')
})

it('does not reuse a cache when a reported directory has been deleted', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  mkdirSync(join(f.root, 'app/reported'))
  const first = await f.manager.prepare(f.signal)
  rmSync(join(f.root, 'app/reported'), { recursive: true })
  // When
  await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(f.counts()).toBe(2)
})

it('rejects generations from another manager without accepting stale ownership', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const first = await f.manager.prepare(f.signal)
  const other = createMdxSourceManager(f.binding)
  // When / Then
  expect(() => other.restartCache(first)).toThrow('does not belong')
  await expect(
    other.refresh({ generation: first, signal: f.signal }),
  ).rejects.toThrow('does not belong')
  await expect(
    f.manager.prepare(AbortSignal.abort(new Error('cancelled'))),
  ).rejects.toThrow('cancelled')
})
