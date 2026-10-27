import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'
import type { Compiler, Configuration, Watching } from 'webpack'

import { DevupUIWebpackPlugin } from '../plugin'

const installed = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const bundled: { webpack(config: Configuration): Compiler } = installed(
  'next/dist/compiled/webpack/webpack',
)

it('retains live ownership when real webpack watch rebuilds and closes normally', async () => {
  // Given
  const root = await mkdtemp(join(tmpdir(), 'devup-owner-watch-'))
  const entry = join(root, 'entry.mjs')
  await writeFile(entry, 'export const value="first"')
  const compiler = bundled.webpack({
    context: root,
    mode: 'development',
    entry: './entry.mjs',
    output: { path: join(root, 'out') },
    plugins: [new DevupUIWebpackPlugin({ watch: true })],
  })
  let watching: Watching | undefined
  let deadline: ReturnType<typeof setTimeout> | undefined
  let first = true
  try {
    // When
    await new Promise<void>((done, reject) => {
      deadline = setTimeout(
        () => reject(new Error('watch completion deadline')),
        30000,
      )
      watching = compiler.watch({}, (error, stats) => {
        if (error) return reject(error)
        if (!stats) return reject(new Error('webpack supplied no watch stats'))
        if (stats.hasErrors())
          return reject(new Error(stats.toString({ all: false, errors: true })))
        if (first) {
          first = false
          writeFile(entry, 'export const value="second"').then(
            () => watching?.invalidate(),
            reject,
          )
        } else {
          readFile(join(root, 'out/main.js'), 'utf8').then((output) => {
            if (output.includes('second')) done()
          }, reject)
        }
      })
    })
    // Then
    expect(await readFile(join(root, 'out/main.js'), 'utf8')).toContain(
      'second',
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
      await new Promise<void>((done, reject) =>
        compiler.close((error) => (error ? reject(error) : done())),
      )
      await rm(root, { recursive: true, force: true })
    }
  }
}, 40000)
