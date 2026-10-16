import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import { installedMdxJsonParser } from '../mdx-options-parser'
import { compileMdx } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'

const roots: string[] = []
afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})

function parserProject(metadata: string, exported: string) {
  const root = mkdtempSync(join(tmpdir(), 'devup-mdx-parser-boundary-'))
  roots.push(root)
  const webpack = join(root, 'node_modules/next/dist/compiled/webpack')
  const parser = join(root, 'node_modules/json-parse-even-better-errors')
  mkdirSync(webpack, { recursive: true })
  mkdirSync(parser, { recursive: true })
  writeFileSync(join(webpack, 'webpack.js'), 'module.exports = {}')
  writeFileSync(join(parser, 'package.json'), metadata)
  writeFileSync(join(parser, 'index.js'), exported)
  return root
}

it.each([
  { metadata: 'null', exported: 'module.exports = value => value' },
  {
    metadata: '{"version":"0.0.0"}',
    exported: 'module.exports = value => value',
  },
  { metadata: '{"version":"3.0.2"}', exported: 'module.exports = {}' },
  {
    metadata: '{"version":"3.0.2"}',
    exported: 'throw new Error("unavailable parser")',
  },
])(
  'refuses an incompatible installed parser package $metadata / $exported',
  ({ metadata, exported }) => {
    // Given
    const root = parserProject(metadata, exported)
    // When
    const parser = installedMdxJsonParser(root)
    // Then
    expect(parser).toBeUndefined()
  },
)

it('preserves a non-Error thrown by an installed parser module boundary', async () => {
  // Given
  const key = 'w21-parser-' + crypto.randomUUID()
  const root = parserProject(
    '{"version":"3.0.2"}',
    `throw Symbol.for(${JSON.stringify(key)})`,
  )
  // When
  const error = await Promise.resolve()
    .then(() => installedMdxJsonParser(root))
    .catch((cause: unknown) => cause)
  // Then
  expect(error).toBe(Symbol.for(key))
})

it('discards preparation after a raw loader catches missing JSON parser admission', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const root = parserProject(
    '{"version":"0.0.0"}',
    'module.exports = value => value',
  )
  const native = createRequire(
    resolve(import.meta.dir, '../../../../apps/landing/package.json'),
  )
  const runner = join(root, 'node_modules/next/dist/compiled/loader-runner')
  mkdirSync(runner, { recursive: true })
  writeFileSync(
    join(runner, 'LoaderRunner.js'),
    `module.exports = require(${JSON.stringify(native.resolve('next/dist/compiled/loader-runner/LoaderRunner.js'))})`,
  )
  const raw = join(root, 'caught-json.cjs')
  writeFileSync(
    raw,
    'module.exports = function() { try { this.getOptions() } catch (cause) { if (!(cause instanceof Error)) throw cause }; module.exports.completed = true; return "# caught missing parser" }',
  )
  // When
  const error = await compileMdx({
    ...request,
    root,
    pipeline: {
      ...request.pipeline,
      loaders: [...request.pipeline.loaders, { loader: raw, options: '{}' }],
    },
  }).catch((cause: unknown) => cause)
  // Then
  expect(error).toMatchObject({
    filename: request.filename,
    cause: { property: 'getOptions JSON parser unavailable', loader: raw },
  })
  const loaded: unknown = native(raw)
  expect(typeof loaded).toBe('function')
  if (typeof loaded !== 'function') throw new TypeError('missing raw module')
  expect(Reflect.get(loaded, 'completed')).toBe(true)
})
