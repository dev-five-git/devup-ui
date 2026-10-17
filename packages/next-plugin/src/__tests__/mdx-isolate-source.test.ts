import { once } from 'node:events'
import { watch, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { MessageChannel } from 'node:worker_threads'

import { expect, it } from 'bun:test'

import { MdxLoaderExecutionError } from '../mdx-invocation'
import {
  checkMdxIsolateScope,
  createMdxIsolateAuthority,
  currentMdxIsolateFailureScope,
  isolatePreparationSteps,
  rememberMdxIsolateFailureScope,
  rememberMdxIsolateScope,
} from '../mdx-isolate-authority'
import { createMdxIsolateRuntime } from '../mdx-isolate-runtime'
import { startMdxIsolateWorker } from '../mdx-isolate-worker'
import { recognizeMdxPrewarmStep } from '../mdx-prewarm-boundary'
import { fixture } from './mdx-boundary-fixture.test'

function runtimeFixture() {
  const f = fixture('@next/mdx/mdx-js-loader')
  const compiler = f.pipeline.loaders[0]
  if (!compiler) throw new TypeError('missing compiler')
  const plugin = join(f.root, 'unique.cjs')
  writeFileSync(plugin, 'module.exports = function() {}')
  const pipeline = {
    ...f.pipeline,
    bundler: 'turbo' as const,
    loaders: [
      {
        ...compiler,
        options: { remarkPlugins: [[plugin, { identity: plugin }]] },
      },
    ],
  }
  return {
    ...f,
    compiler,
    pipeline,
    demand: { run: 'one', filename: f.filename, pipeline, timeoutMs: 5000 },
  }
}

it('reuses the actual source runtime result only within one run/resource', async () => {
  // Given
  const f = runtimeFixture()
  const compile = createMdxIsolateRuntime(f.root)
  const first = await compile(f.demand)
  // When
  const repeated = await compile(f.demand)
  const other = join(f.root, 'second.mdx')
  writeFileSync(other, '# original')
  expect(await compile({ ...f.demand, filename: other })).toMatchObject({
    kind: 'prepared',
  })
  const next = await compile({ ...f.demand, run: 'two' })
  // Then
  expect(first.kind).toBe('prepared')
  expect(repeated).toBe(first)
  expect(next).toEqual(first)
  expect(next).not.toBe(first)
})

it('keeps source-proven parser diagnostics resource-local and recovers in the same runtime', async () => {
  // Given
  const f = fixture('@next/mdx/mdx-js-loader')
  const compile = createMdxIsolateRuntime(f.root)
  const demand = {
    run: 'one',
    filename: f.filename,
    pipeline: f.pipeline,
    timeoutMs: 5000,
  }
  writeFileSync(f.filename, '<Unclosed')
  const first = await compile(demand)
  writeFileSync(f.filename, '# corrected')
  // When
  const next = await compile({
    ...demand,
    run: 'two',
  })
  // Then
  expect(first.kind).toBe('failed')
  expect(next.kind).toBe('prepared')
})

it.each(['{', 'import x from', '<a></b>'])(
  'recovers after an installed parser family rejects %s',
  async (source) => {
    // Given
    const f = fixture('@next/mdx/mdx-js-loader', {
      remarkPlugins: [],
      rehypePlugins: [],
      recmaPlugins: [],
    })
    const compile = createMdxIsolateRuntime(f.root)
    writeFileSync(f.filename, source)
    const demand = {
      run: 'one',
      filename: f.filename,
      pipeline: f.pipeline,
      timeoutMs: 5000,
    }
    const rejected = await compile(demand)
    writeFileSync(f.filename, '# corrected')
    // When
    const corrected = await compile({ ...demand, run: 'corrected' })
    // Then
    expect(rejected.kind).toBe('failed')
    expect(corrected.kind).toBe('prepared')
  },
)

it.each(['remarkPlugins', 'rehypePlugins', 'recmaPlugins', 'raw'])(
  'quarantines ordinary syntax failure with configured %s',
  async (kind) => {
    // Given
    const f = runtimeFixture()
    const plugin = join(f.root, 'unique.cjs')
    const raw = join(f.root, 'raw.cjs')
    writeFileSync(raw, 'module.exports = function(source) { return source }')
    const pipeline = {
      ...f.pipeline,
      loaders: [
        { ...f.compiler, options: kind === 'raw' ? {} : { [kind]: [plugin] } },
        ...(kind === 'raw' ? [{ loader: raw }] : []),
      ],
    }
    const demand = { ...f.demand, pipeline }
    const compile = createMdxIsolateRuntime(f.root)
    writeFileSync(f.filename, '<Unclosed')
    // When
    const failed = await compile(demand)
    writeFileSync(f.filename, '# corrected')
    const later = await compile({ ...demand, run: 'corrected' })
    // Then
    expect(failed.kind).toBe('failed')
    expect('recoverable' in failed && failed.recoverable).not.toBe(true)
    expect(later.kind).toBe('failed')
  },
)

it('quarantines parser rejection when plugin construction also invalidated current native H', async () => {
  // Given
  const f = runtimeFixture()
  writeFileSync(
    join(f.root, 'unique.cjs'),
    `module.exports = function(options) { options.identity = 'changed' }`,
  )
  writeFileSync(f.filename, '<Unclosed')
  const compile = createMdxIsolateRuntime(f.root)
  // When
  const outcome = await compile(f.demand)
  writeFileSync(f.filename, '# corrected')
  const later = await compile({ ...f.demand, run: 'later' })
  // Then
  expect(outcome.kind).toBe('failed')
  expect('recoverable' in outcome && outcome.recoverable).not.toBe(true)
  expect(later.kind).toBe('failed')
})

it('locates an unavailable compiler before issuing preparation authority', async () => {
  // Given
  const f = runtimeFixture()
  const compile = createMdxIsolateRuntime(f.root)
  // When
  const output = await compile({
    ...f.demand,
    pipeline: { ...f.pipeline, loaders: [] },
  })
  // Then
  expect(output).toMatchObject({
    kind: 'failed',
    error: { filename: f.filename, line: 1, column: 1 },
  })
  expect(await compile({ ...f.demand, run: 'after-fatal' })).toMatchObject({
    kind: 'failed',
  })
})

it.each([
  '@next/mdx/mdx-js-loader',
  './node_modules/@next/mdx/mdx-js-loader.js',
])(
  'resolves the configured compiler without changing physical identity: %s',
  async (loader) => {
    // Given
    const f = runtimeFixture()
    const compiler = f.pipeline.loaders[0]
    if (!compiler) throw new TypeError('missing fixture compiler')
    const compile = createMdxIsolateRuntime(f.root)
    // When
    const outcome = await compile({
      ...f.demand,
      pipeline: { ...f.pipeline, loaders: [{ ...compiler, loader }] },
    })
    // Then
    expect(outcome).toMatchObject({
      kind: 'prepared',
      output: { filename: f.filename, map: undefined },
    })
  },
)

it('issues source authority separately from receiving receipts and rejects mismatched physical scope', () => {
  // Given
  const f = runtimeFixture()
  const step = recognizeMdxPrewarmStep(f.compiler.loader)
  if (!step) throw new TypeError('unrecognized installed fixture')
  const context = createMdxIsolateAuthority(f.root, f.compiler.loader)
  const steps = new Map([[0, { ...step, compiler: undefined }]])
  // When
  const authorized = isolatePreparationSteps({ root: f.root, context, steps })
  // Then
  expect(authorized.get(0)?.compiler).toBeNull()
  expect(context).toEqual({ owner: {}, generation: {}, sourceMap: false })
  expect(() =>
    isolatePreparationSteps({ root: 'different-root', context, steps }),
  ).toThrow('authority mismatch')
  expect(() =>
    isolatePreparationSteps({
      root: f.root,
      context,
      steps: new Map([[0, { ...step, module: 'other', compiler: undefined }]]),
    }),
  ).toThrow('physical compiler mismatch')
  expect(() => createMdxIsolateAuthority(f.root, 'unrecognized')).toThrow(
    'source-verified',
  )
})

it('drives the worker message boundary over a real channel without transporting opaque context', async () => {
  // Given
  const f = runtimeFixture()
  const { port1, port2 } = new MessageChannel()
  try {
    startMdxIsolateWorker(port1, f.root)
    const response = once(port2, 'message')
    // When
    port2.postMessage(f.demand)
    const [output] = await response
    // Then
    expect(output).toMatchObject({
      kind: 'prepared',
      output: { filename: f.filename, map: undefined },
    })
    const checked = once(port2, 'message')
    port2.postMessage({
      operation: 'check',
      run: 'one',
      filename: f.filename,
      timeoutMs: 5000,
    })
    expect((await checked)[0]).toEqual({ run: 'one', filename: f.filename })
    const sealed = once(port2, 'message')
    port2.postMessage({
      operation: 'finalize',
      run: 'one',
      filename: f.filename,
      timeoutMs: 5000,
    })
    expect((await sealed)[0]).toMatchObject({
      terminal: [{ run: 'one', filename: f.filename }],
    })
    const retired = once(port2, 'message')
    port2.postMessage({
      operation: 'retire',
      run: 'one',
      filename: f.filename,
      timeoutMs: 5000,
    })
    expect((await retired)[0]).toEqual({ run: 'one', filename: f.filename })
    const closed = once(port2, 'message')
    port2.postMessage({
      operation: 'check',
      run: 'one',
      filename: f.filename,
      timeoutMs: 5000,
    })
    expect((await closed)[0]).toMatchObject({ error: { filename: f.filename } })
    expect(() => startMdxIsolateWorker(port1, null)).toThrow('project root')
  } finally {
    port1.close()
    port2.close()
  }
})

it('does not revive retired source-runtime jobs or expose a check for an absent resource', async () => {
  // Given
  const f = runtimeFixture()
  const compile = createMdxIsolateRuntime(f.root)
  await compile(f.demand)
  // When
  compile.retire('one')
  const revived = await compile(f.demand)
  // Then
  expect(revived).toMatchObject({ kind: 'failed', recoverable: true })
  expect(compile.check('absent', f.filename)).toMatchObject({
    error: { filename: f.filename },
  })
})

it('requires the original isolate-local scope and calls it on each check', () => {
  // Given
  const f = runtimeFixture()
  const context = createMdxIsolateAuthority(f.root, f.compiler.loader)
  const output = {
    filename: f.filename,
    source: 'compiled',
    map: undefined,
    dependencies: [],
    contextDependencies: [],
    missingDependencies: [],
    buildDependencies: [],
    dependencyReports: [],
  }
  let current = true
  let calls = 0
  const scope = () => {
    calls++
    return current
  }
  expect(() => rememberMdxIsolateScope(context, output, undefined)).toThrow(
    'unavailable',
  )
  rememberMdxIsolateScope(context, output, scope)
  // When
  checkMdxIsolateScope(output)
  current = false
  // Then
  expect(() => checkMdxIsolateScope(output)).toThrow(
    'scope/configuration changed',
  )
  expect(calls).toBe(2)
})

it('does not grant parser recovery from a missing, other-loader or throwing native scope', () => {
  // Given
  const f = runtimeFixture()
  const context = createMdxIsolateAuthority(f.root, f.compiler.loader)
  const cause = new Error('original diagnostic')
  const failure = new MdxLoaderExecutionError('rule', f.compiler.loader, cause)
  // When / Then
  expect(currentMdxIsolateFailureScope(null)).toBe(false)
  expect(currentMdxIsolateFailureScope(cause)).toBe(false)
  rememberMdxIsolateFailureScope({
    context,
    failure,
    compilerPath: 'other-loader',
    scope: () => true,
  })
  expect(currentMdxIsolateFailureScope(cause)).toBe(false)
  rememberMdxIsolateFailureScope({
    context,
    failure,
    compilerPath: f.compiler.loader,
    scope: () => {
      throw new TypeError('native scope rejected')
    },
  })
  expect(currentMdxIsolateFailureScope(cause)).toBe(false)
  const unknown: unknown = { error: 'opaque native failure' }
  rememberMdxIsolateFailureScope({
    context,
    failure,
    compilerPath: f.compiler.loader,
    scope: () => {
      throw unknown
    },
  })
  let observed: unknown
  try {
    currentMdxIsolateFailureScope(cause)
  } catch (error) {
    observed = error
  }
  expect(observed).toBe(unknown)
})

it('retains a source change during compilation as a provisional freshness failure', async () => {
  // Given
  const f = runtimeFixture()
  const started = join(f.root, 'started')
  const release = join(f.root, 'release')
  const raw = join(f.root, 'pause.cjs')
  writeFileSync(started, '')
  writeFileSync(release, '')
  writeFileSync(
    raw,
    `module.exports = function(source) { const fs = require('node:fs'); const callback = this.async(); const watcher = fs.watch(${JSON.stringify(release)}, () => { watcher.close(); callback(null, source) }); fs.writeFileSync(${JSON.stringify(started)}, 'started') }`,
  )
  const watcher = watch(started)
  try {
    const observed = once(watcher, 'change')
    const compile = createMdxIsolateRuntime(f.root)
    const pending = compile({
      ...f.demand,
      pipeline: {
        ...f.pipeline,
        loaders: [...f.pipeline.loaders, { loader: raw }],
      },
    })
    await observed
    // When
    writeFileSync(f.filename, '# changed during compile')
    writeFileSync(release, 'release')
    const outcome = await pending
    // Then
    expect(outcome).toMatchObject({
      kind: 'failed',
      error: { filename: f.filename, line: 1, column: 1 },
    })
  } finally {
    watcher.close()
  }
})
