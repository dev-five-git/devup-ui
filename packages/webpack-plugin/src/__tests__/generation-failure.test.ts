import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import type { Compiler, Configuration, Stats, Watching } from 'webpack'

import { compilerScope, createWebpackGeneration } from '../build-scope'
import { DevupUIWebpackPlugin } from '../plugin'

const installed = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const bundled: { webpack(config: Configuration): Compiler } = installed(
  'next/dist/compiled/webpack/webpack',
)

function closeCompiler(compiler: Compiler): Promise<void> {
  return new Promise((done, reject) =>
    compiler.close((error) => (error ? reject(error) : done())),
  )
}

function seedSheet(compiler: Compiler): string {
  const scope = compilerScope(compiler)
  if (!scope) throw new Error('native compiler has no owner scope')
  return scope.run(() => {
    wasm.codeExtract(
      'previous.tsx',
      "import {Box} from '@devup-ui/react';export const x=<Box bg='red'/>",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    return wasm.getCss(null, false)
  })
}

it('discards the shared owner when native production fails before compilation', async () => {
  // Given
  const root = await mkdtemp(join(tmpdir(), 'devup-owner-fatal-'))
  await writeFile(join(root, 'entry.mjs'), 'export const value="ordinary"')
  const owner = createWebpackGeneration()
  const fault = new Error('fatal-beforeCompile-marker')
  const compiler = bundled.webpack({
    context: root,
    mode: 'production',
    optimization: { minimize: false },
    entry: './entry.mjs',
    output: { path: join(root, 'out') },
    plugins: [new DevupUIWebpackPlugin({}, { owner, complete: false })],
  })
  compiler.hooks.beforeCompile.tap('FatalOwnerRegression', () => {
    throw fault
  })
  expect(seedSheet(compiler)).toContain('background:red')
  try {
    // When
    const observed = await new Promise<Error | null | undefined>((done) =>
      compiler.run((error) => done(error)),
    )
    await closeCompiler(compiler)
    // Then
    expect(observed).toBe(fault)
    expect(owner.disposed).toBe(true)
    expect(wasm.getCss(null, false)).not.toContain('background:red')
  } finally {
    await closeCompiler(compiler)
    await rm(root, { recursive: true, force: true })
  }
})

it('discards the shared owner when native production reports compilation errors', async () => {
  // Given
  const root = await mkdtemp(join(tmpdir(), 'devup-owner-production-error-'))
  await writeFile(join(root, 'entry.txt'), 'export const value = ;')
  const owner = createWebpackGeneration()
  const compiler = bundled.webpack({
    context: root,
    mode: 'production',
    optimization: { minimize: false },
    entry: './entry.txt',
    output: { path: join(root, 'out') },
    module: { rules: [{ test: /\.txt$/, type: 'javascript/auto' }] },
    plugins: [new DevupUIWebpackPlugin({}, { owner, complete: false })],
  })
  try {
    // When
    const stats = await new Promise<Stats>((done, reject) =>
      compiler.run((error, result) => {
        if (error) return reject(error)
        if (!result) return reject(new Error('webpack supplied no stats'))
        done(result)
      }),
    )
    // Then
    expect(stats.hasErrors()).toBe(true)
    expect(owner.disposed).toBe(true)
  } finally {
    await closeCompiler(compiler)
    await rm(root, { recursive: true, force: true })
  }
})

it.each([{}, { watch: true }])(
  'recovers watch (%j)',
  async (options) => {
    // Given
    const root = await mkdtemp(join(tmpdir(), 'devup-owner-watch-failure-'))
    const entry = join(root, 'entry.mjs')
    await writeFile(entry, 'export const value="first"')
    const owner = createWebpackGeneration()
    const fault = new Error('watch-beforeCompile-marker')
    const compiler = bundled.webpack({
      context: root,
      mode: 'development',
      entry: './entry.mjs',
      output: { path: join(root, 'out') },
      plugins: [new DevupUIWebpackPlugin(options, { owner, complete: true })],
    })
    const scope = compilerScope(compiler)
    if (!scope) throw new Error('native compiler has no owner scope')
    let watching: Watching | undefined
    let deadline: ReturnType<typeof setTimeout> | undefined
    let stage = 0
    let previous = ''
    compiler.hooks.beforeCompile.tap('WatchOwnerFailureControl', () => {
      if (stage === 1) throw fault
    })
    try {
      // When: successful watch -> fatal error -> compilation error -> recovery
      await new Promise<void>((done, reject) => {
        deadline = setTimeout(
          () => reject(new Error('watch failure recovery deadline')),
          30000,
        )
        watching = compiler.watch({}, (error, stats) => {
          try {
            if (stage === 1) {
              expect(error).toBe(fault)
              expect(owner.disposed).toBe(false)
              expect(scope.run(() => wasm.getCss(null, false))).toBe(previous)
              stage = 2
              writeFile(entry, 'export const value = ;').then(
                () => watching?.invalidate(),
                reject,
              )
              return
            }
            if (error) return reject(error)
            if (!stats)
              return reject(new Error('webpack supplied no watch stats'))
            if (stage === 0) {
              expect(stats.hasErrors()).toBe(false)
              previous = seedSheet(compiler)
              expect(previous).toContain('background:red')
              stage = 1
              watching?.invalidate()
            } else if (stage === 2 && stats.hasErrors()) {
              expect(owner.disposed).toBe(false)
              expect(scope.run(() => wasm.getCss(null, false))).toBe(previous)
              stage = 3
              writeFile(entry, 'export const value="recovered"').then(
                () => watching?.invalidate(),
                reject,
              )
            } else if (stage === 3 && !stats.hasErrors()) {
              readFile(join(root, 'out/main.js'), 'utf8').then((output) => {
                if (output.includes('recovered')) done()
              }, reject)
            }
          } catch (cause) {
            reject(cause)
          }
        })
      })
      // Then
      expect(owner.disposed).toBe(false)
      expect(scope.run(() => wasm.getCss(null, false))).toBe(previous)
      expect(await readFile(join(root, 'out/main.js'), 'utf8')).toContain(
        'recovered',
      )
    } finally {
      clearTimeout(deadline)
      try {
        const active = watching
        if (active)
          await new Promise<void>((done, reject) =>
            active.close((error) => (error ? reject(error) : done())),
          )
      } finally {
        await closeCompiler(compiler)
        await rm(root, { recursive: true, force: true })
      }
    }
  },
  40000,
)
