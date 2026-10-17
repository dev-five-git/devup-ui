import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { expect, it } from 'bun:test'

import { isMdxRecord } from '../mdx-pipeline'
import { isRunLoaders, runMdxLoaders } from '../mdx-prepare-runner'

const root = resolve(import.meta.dir, '../../../../apps/landing')
const installed = createRequire(join(root, 'package.json'))
const runner: unknown = installed(
  'next/dist/compiled/loader-runner/LoaderRunner.js',
)
if (!isMdxRecord(runner) || !isRunLoaders(runner.runLoaders))
  throw new TypeError('Installed loader runner unavailable')
const runLoaders = runner.runLoaders
const downstream = fileURLToPath(
  new URL('./webpack-resource-downstream.cjs', import.meta.url),
)
const pitch = fileURLToPath(
  new URL('./webpack-resource-pitch.cjs', import.meta.url),
)

it('passes actual disk bytes through the downstream fixture when native normal execution runs', async () => {
  // Given
  const input = readFileSync(downstream, 'utf8')
  // When
  const output = await runMdxLoaders({
    root,
    filename: downstream,
    loaders: [{ loader: downstream }],
    context: { owner: {}, generation: {} },
    steps: new Map(),
    signal: new AbortController().signal,
    timeoutMs: 1000,
    runLoaders,
  })
  // Then
  expect(output.source).toBe(input)
})

it('substitutes before resource reads when the downstream pitch short-circuits native execution', async () => {
  // Given: no resource exists, so normal disk input cannot produce this result.
  const filename = join(import.meta.dir, 'unread-resource-for-pitch.js')
  // When
  const output = await runMdxLoaders({
    root,
    filename,
    loaders: [{ loader: downstream }, { loader: pitch }],
    context: { owner: {}, generation: {} },
    steps: new Map(),
    signal: new AbortController().signal,
    timeoutMs: 1000,
    runLoaders,
  })
  // Then
  expect(output.source).toBe('export default "replacement"')
})
