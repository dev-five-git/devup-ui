import { readFile, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { isMdxRecord } from '../mdx-pipeline'
import { isRunLoaders, runMdxLoaders } from '../mdx-prepare-runner'
import { fixture } from './mdx-boundary-fixture.test'

const installed = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const native: unknown = installed(
  'next/dist/compiled/loader-runner/LoaderRunner.js',
)
if (!isMdxRecord(native) || !isRunLoaders(native.runLoaders))
  throw new TypeError('missing installed runner')
const nativeRun = native.runLoaders

it.each([true, false])(
  'checks successful native cacheability=%s without a capture seam',
  async (cacheable) => {
    // Given
    const request = fixture('@mdx-js/loader')
    const compiler = join(request.root, 'compiler.cjs')
    writeFileSync(
      compiler,
      'module.exports = function() { return "native source" }',
    )
    // When
    const result = await runMdxLoaders({
      ...request,
      timeoutMs: 1000,
      loaders: [{ loader: compiler }],
      steps: new Map(),
      runLoaders(options, callback) {
        nativeRun(options, (error, result) =>
          callback(
            error,
            isMdxRecord(result) ? { ...result, cacheable } : result,
          ),
        )
      },
    }).catch((cause: unknown) => cause)
    // Then
    if (cacheable) expect(result).toMatchObject({ source: 'native source' })
    else expect(result).toBeInstanceOf(TypeError)
  },
)

it('rejects an abnormal native result getter through the promise instead of throwing from a native callback', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const compiler = join(request.root, 'compiler.cjs')
  const own = join(request.root, 'own.cjs')
  writeFileSync(
    compiler,
    'module.exports = function() { return "native source" }',
  )
  writeFileSync(
    own,
    'module.exports = function() { throw new Error("own must only capture") }',
  )
  const token = Symbol('native result access')
  // When
  const result = await runMdxLoaders({
    ...request,
    timeoutMs: 1000,
    loaders: [{ loader: own }, { loader: compiler }],
    steps: new Map(),
    seam: {
      ruleKey: 'boundary',
      compilerIndex: 1,
      ownIndex: 0,
      compilerPath: compiler,
      loaderCount: 2,
      bridge: false,
    },
    runLoaders(options, callback) {
      nativeRun(options, (error, result) => {
        if (!isMdxRecord(result)) throw new TypeError('missing native result')
        callback(
          error,
          Object.defineProperty({ ...result }, 'unexpected', {
            enumerable: true,
            get() {
              throw token
            },
          }),
        )
        callback(null, result)
      })
    },
  }).catch((cause: unknown) => cause)
  // Then
  expect(result).toBe(token)
  await expect(
    new Promise((done, reject) => {
      nativeRun(
        {
          resource: request.filename,
          loaders: [{ loader: own }, { loader: compiler }],
          readResource: readFile,
          context: {},
        },
        (error, output) => (error ? reject(error) : done(output)),
      )
    }),
  ).rejects.toThrow('own must only capture')
})

it('never executes a normal whose native module load completes after invocation cancellation', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const controller = new AbortController()
  const started = Promise.withResolvers<void>()
  const release = Promise.withResolvers<void>()
  const finished = Promise.withResolvers<unknown>()
  let executions = 0
  const controlFile = join(request.root, 'load-control.cjs')
  const compiler = join(request.root, 'late-compiler.mjs')
  writeFileSync(controlFile, 'module.exports = {}')
  const control: unknown = installed(controlFile)
  if (!isMdxRecord(control)) throw new TypeError('invalid load control')
  Reflect.set(control, 'started', started.resolve)
  Reflect.set(control, 'release', release.promise)
  Reflect.set(control, 'executed', () => {
    executions++
  })
  writeFileSync(
    compiler,
    'import control from "./load-control.cjs"; control.started(); await control.release; export default function(source) { control.executed(); return source }',
  )
  const pending = runMdxLoaders({
    ...request,
    signal: controller.signal,
    timeoutMs: 1000,
    loaders: [{ loader: compiler, type: 'module' }],
    steps: new Map(),
    seam: {
      ruleKey: 'late-module',
      compilerIndex: 0,
      compilerPath: compiler,
      loaderCount: 1,
      bridge: false,
    },
    runLoaders(options, callback) {
      nativeRun(options, (error, result) => {
        finished.resolve(error)
        callback(error, result)
      })
    },
  })
  await started.promise
  // When
  const reason = new Error('cancel native load')
  controller.abort(reason)
  const result = await pending.catch((cause: unknown) => cause)
  release.resolve()
  const nativeError = await finished.promise
  // Then
  expect(result).toBe(reason)
  expect(nativeError).toMatchObject({
    property: 'context',
    reason: 'invocation has completed',
  })
  expect(executions).toBe(0)
  const fresh = await runMdxLoaders({
    ...request,
    timeoutMs: 1000,
    loaders: [{ loader: compiler, type: 'module' }],
    steps: new Map(),
    runLoaders: nativeRun,
  })
  expect(fresh.source).toBe('# original')
  expect(executions).toBe(1)
})
