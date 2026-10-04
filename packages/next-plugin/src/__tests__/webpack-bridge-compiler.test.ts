import { createRequire } from 'node:module'

import { expect, it } from 'bun:test'
import type { Compiler, Stats } from 'webpack'

import { compiler, webpack } from './webpack-bridge-fixture'

const adapter: {
  readonly run: (compiler: Pick<Compiler, 'run'>) => Promise<Stats>
  readonly close: (compiler: Pick<Compiler, 'close'>) => Promise<void>
} = createRequire(import.meta.url)('./webpack-bridge-compiler.cjs')

it('returns real stats when the compiler completes without errors', async () => {
  // Given
  const instance = compiler({})
  const stats = new webpack.Stats(
    new webpack.Compilation(instance, instance.newCompilationParams()),
  )
  // When
  const result = await adapter.run({ run: (callback) => callback(null, stats) })
  // Then
  expect(result).toBe(stats)
})

it('retains the located cause when the compiler callback fails', async () => {
  // Given
  const cause = new Error('/app/page.mdx:2:1: failed')
  // When / Then
  await expect(
    adapter.run({ run: (callback) => callback(cause) }),
  ).rejects.toBe(cause)
})

it('rejects when webpack completes with module errors', async () => {
  // Given
  const instance = compiler({})
  const compilation = new webpack.Compilation(
    instance,
    instance.newCompilationParams(),
  )
  compilation.errors.push(new webpack.WebpackError('module failure'))
  const stats = new webpack.Stats(compilation)
  // When / Then
  await expect(
    adapter.run({ run: (callback) => callback(null, stats) }),
  ).rejects.toBeInstanceOf(Error)
})

it('settles when compiler close succeeds', async () => {
  // Given / When
  const result = await adapter.close({ close: (callback) => callback(null) })
  // Then
  expect(result).toBeUndefined()
})

it('retains the cause when compiler close fails', async () => {
  // Given
  const cause = new Error('close failed')
  // When / Then
  await expect(
    adapter.close({ close: (callback) => callback(cause) }),
  ).rejects.toBe(cause)
})
