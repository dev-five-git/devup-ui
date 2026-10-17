import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import type { MdxPipeline } from '../mdx-pipeline'
import { compileMdx, MdxCompileError } from '../mdx-prepare'
import {
  captureMdxTurboContext,
  provenMdxTurboCompiler,
} from '../mdx-turbo-context'
import { markerFixture } from './mdx-marker-fixture'

it.each([
  'omitted',
  'undefined',
  'forged',
  'null',
  'mode',
  'maps',
  'root',
  'webpack',
] as const)(
  'refuses unknown or mismatched marker evidence: %s',
  async (form) => {
    // Given: supplemental boundary controls, not receiving-worker evidence.
    const f = markerFixture()
    let context: Parameters<typeof compileMdx>[0]['context'] = f.context
    let root = f.root
    let pipeline: MdxPipeline = f.pipeline
    switch (form) {
      case 'omitted':
        context = undefined
        break
      case 'undefined':
        context = { owner: {}, generation: {}, compiler: undefined }
        break
      case 'forged':
        context = { owner: {}, generation: {}, native: {}, sourceMap: true }
        break
      case 'null':
        context = { ...f.context }
        Reflect.set(context, 'compiler', null)
        break
      case 'mode':
        context = { ...f.context, mode: 'production' }
        break
      case 'maps':
        context = { ...f.context, sourceMap: false }
        break
      case 'root':
        root = join(f.root, 'app')
        break
      case 'webpack':
        pipeline = { ...pipeline, bundler: 'webpack' }
        break
      default:
        form satisfies never
    }
    // When
    const refused = await compileMdx({ ...f, context, pipeline, root }).catch(
      (error: unknown) => error,
    )
    // Then
    expect(refused).toBeInstanceOf(MdxCompileError)
    expect(refused).toMatchObject({
      filename: f.filename,
      cause: { option: '_compiler' },
    })
  },
)

it.each(['unknown', 'compiler', 'mode', 'tag', 'maps', 'root'] as const)(
  'rejects unsupported receiving facts without issuing a receipt: %s',
  (form) => {
    // Given
    const f = markerFixture()
    const receiving: Record<string, unknown> = { ...f.receiving }
    let loader = f.loader.loader
    switch (form) {
      case 'unknown':
        loader = join(f.root, 'unknown.cjs')
        break
      case 'compiler':
        receiving._compiler = undefined
        break
      case 'mode':
        receiving.mode = undefined
        break
      case 'tag':
        receiving._module = {}
        break
      case 'maps':
        Object.defineProperty(receiving, 'sourceMap', {
          get() {
            throw new Error('must not evaluate getter')
          },
        })
        break
      case 'root':
        delete receiving.rootContext
        break
      default:
        form satisfies never
    }
    // When / Then
    expect(() => captureMdxTurboContext(loader, receiving)).toThrow(TypeError)
  },
)

it('rejects another installed Next version and another local compiler module', () => {
  // Given
  const f = markerFixture()
  const root = mkdtempSync(join(tmpdir(), 'devup-mdx-profile-'))
  mkdirSync(join(root, 'node_modules/next'), { recursive: true })
  writeFileSync(
    join(root, 'node_modules/next/package.json'),
    '{"version":"unsupported"}',
  )
  try {
    // When / Then
    expect(() =>
      captureMdxTurboContext(f.loader.loader, {
        ...f.receiving,
        rootContext: root,
      }),
    ).toThrow(TypeError)
    expect(
      provenMdxTurboCompiler(f.context, f.root, 'another-module'),
    ).toBeUndefined()
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
