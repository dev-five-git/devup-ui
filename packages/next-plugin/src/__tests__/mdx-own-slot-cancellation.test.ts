import { expect, it } from 'bun:test'

import { compileMdx, createMdxDeadline, MdxCompileError } from '../mdx-prepare'
import { fullFixture } from './mdx-own-slot-fixture'

it('publishes nothing when an async full invocation exceeds its bounded deadline', async () => {
  // Given
  const request = fullFixture('this.async()')
  // When / Then
  await expect(
    compileMdx({ ...request, deadline: createMdxDeadline(30_000, 20) }),
  ).rejects.toMatchObject({ filename: request.filename })
})

it('ignores late full-invocation callbacks when caller cancellation wins', async () => {
  // Given
  const started =
    Promise.withResolvers<(error: unknown, source: unknown) => void>()
  const controller = new AbortController()
  const request = fullFixture('this.getOptions().started(this.async())')
  const [own, compiler, raw] = request.invocation.loaders
  if (!own || !compiler || !raw) throw new TypeError('missing original slots')
  const invocation = {
    ...request.invocation,
    loaders: [own, compiler, { ...raw, options: { started: started.resolve } }],
  }
  const pending = compileMdx({
    ...request,
    invocation,
    signal: controller.signal,
  })
  const callback = await started.promise
  // When
  controller.abort(new Error('cancelled actual invocation'))
  callback(null, Buffer.from('# late callback'))
  // Then
  await expect(pending).rejects.toMatchObject({ filename: request.filename })
})

it('publishes nothing when cancellation occurs after the private stop but before result construction', async () => {
  // Given
  const controller = new AbortController()
  const request = fullFixture(
    'const abort = this.getOptions().abort; this.callback(null, source, map); abort()',
  )
  const [own, compiler, raw] = request.invocation.loaders
  if (!own || !compiler || !raw) throw new TypeError('missing original slots')
  const reason = new Error('cancelled before publication')
  const invocation = {
    ...request.invocation,
    compilerIndex: 2,
    loaders: [
      own,
      { ...raw, options: { abort: () => controller.abort(reason) } },
      compiler,
    ],
  }
  // When
  const error = await compileMdx({
    ...request,
    invocation,
    signal: controller.signal,
  }).catch((cause: unknown) => cause)
  // Then
  if (!(error instanceof MdxCompileError))
    throw new TypeError('expected cancellation failure')
  expect(error.cause).toBe(reason)
})

it('keeps the cancellation cause when a full invocation is already aborted', async () => {
  // Given
  const request = fullFixture('throw new Error("must not run")')
  const reason = new Error('pre-aborted full invocation')
  // When
  const error = await compileMdx({
    ...request,
    signal: AbortSignal.abort(reason),
  }).catch((cause: unknown) => cause)
  // Then
  if (!(error instanceof MdxCompileError))
    throw new TypeError('expected cancellation failure')
  expect(error.cause).toBe(reason)
})
