import { expect, it } from 'bun:test'

import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fullFixture } from './mdx-own-slot-fixture'

it.each(['pitch', 'async', 'own-capture', 'deferred'])(
  'retains first occurrence refusal through %s completion',
  async (phase) => {
    // Given
    const probe =
      'try { void this._compilation } catch (error) { if (!(error instanceof Error)) throw error }'
    const body =
      phase === 'async'
        ? `const done = this.async(); queueMicrotask(() => { ${probe}; done(null, source, map) })`
        : phase === 'deferred'
          ? `queueMicrotask(() => { ${probe} }); this.callback(null, source, map)`
          : `${phase === 'pitch' ? '' : probe}; this.callback(null, source, map)`
    const request = fullFixture(body, phase === 'pitch' ? probe : '')
    // When
    const result = await compileMdx(request).catch((error: unknown) => error)
    // Then
    expect(result).toBeInstanceOf(MdxCompileError)
    expect(result).toMatchObject({
      filename: request.filename,
      cause: {
        property: '_compilation',
        index: 2,
        phase: phase === 'pitch' ? 'pitch' : 'normal',
      },
    })
  },
)

it('keeps cacheability refusal after a caught cacheable(false) and clearDependencies', async () => {
  // Given
  const request = fullFixture(
    'try { this.cacheable(false) } catch (error) { if (!(error instanceof Error)) throw error }; this.clearDependencies(); return source',
  )
  // When
  const result = await compileMdx(request).catch((error: unknown) => error)
  // Then
  expect(result).toMatchObject({
    filename: request.filename,
    cause: { property: 'cacheable(false)', index: 2 },
  })
})

it('accepts cacheable(true) during its original native occurrence', async () => {
  // Given
  const request = fullFixture('this.cacheable(true); return source')
  // When
  const output = await compileMdx(request)
  // Then
  expect(output.source).toContain('original')
  expect(output.dependencies).toContain(request.filename)
})

it('keeps native callback identity when async completion is requested', async () => {
  // Given
  const request = fullFixture(
    'const done = this.async(); if (done !== this.callback) throw new Error("callback identity changed"); queueMicrotask(() => done(null, source, map))',
  )
  // When
  const output = await compileMdx(request)
  // Then
  expect(output.source).toContain('original')
})

it('keeps an occurrence active through a native promise-returning normal until its original completion', async () => {
  // Given
  const request = fullFixture(
    'return Promise.resolve().then(() => { this.addDependency(this.resourcePath + ".promise"); return source })',
  )
  // When
  const output = await compileMdx(request)
  // Then
  expect(output.source).toContain('original')
  expect(output.dependencies).toContain(request.filename + '.promise')
})

it('stickily refuses construction of a context API even when the loader catches it', async () => {
  // Given
  const request = fullFixture(
    'try { new this.getOptions() } catch (error) { if (!(error instanceof Error)) throw error }; return source',
  )
  // When
  const error = await compileMdx(request).catch((cause: unknown) => cause)
  // Then
  expect(error).toMatchObject({
    filename: request.filename,
    cause: { property: 'getOptions.construct', index: 2, phase: 'normal' },
  })
})

it('keeps the original occurrence after async refusal and a later abort', async () => {
  // Given
  const controller = new AbortController()
  const request = fullFixture(
    'const options = this.getOptions(); const done = this.async(); queueMicrotask(() => { try { void this._compiler } catch (error) { if (!(error instanceof Error)) throw error }; options.abort(); done(null, source, map) })',
  )
  const [own, compiler, raw] = request.invocation.loaders
  // When
  const result = await compileMdx({
    ...request,
    signal: controller.signal,
    invocation: {
      ...request.invocation,
      loaders: [
        own,
        compiler,
        {
          ...raw,
          options: { abort: () => controller.abort(new Error('later abort')) },
        },
      ],
    },
  }).catch((error: unknown) => error)
  // Then
  expect(result).toMatchObject({
    filename: request.filename,
    cause: { property: '_compiler', index: 2, phase: 'normal' },
  })
})
