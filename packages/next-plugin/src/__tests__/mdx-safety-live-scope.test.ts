import { mkdirSync, readFileSync, symlinkSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { rememberMdxCacheAdmission } from '../mdx-live-admission'
import { compileMdx, MdxCompileError } from '../mdx-prepare'
import {
  captureMdxCacheIdentity,
  compatibleMdxCache,
} from '../mdx-source-cache'
import { fixture, type RemarkTree } from './mdx-boundary-fixture.test'
import { nativePreparationContext, withNativeMdx } from './mdx-safety-native'

it('misses live compiled bytes when the admitted installed ESM source no longer has its verified identity', async () => {
  // Given
  const request = fixture('@mdx-js/loader', { format: 'mdx' })
  const original = request.pipeline.loaders[0]
  if (!original) throw new TypeError('missing installed compiler')
  const directory = join(request.root, '@mdx-js/loader')
  mkdirSync(join(directory, 'lib'), { recursive: true })
  for (const path of ['package.json', 'index.cjs', 'lib/index.js'])
    writeFileSync(
      join(directory, path),
      readFileSync(join(dirname(original.loader), path)),
    )
  symlinkSync(
    resolve(dirname(original.loader), '../..'),
    join(directory, 'node_modules'),
    'junction',
  )
  const pipeline = {
    ...request.pipeline,
    loaders: [{ ...original, loader: join(directory, 'index.cjs') }],
  }
  const context = {
    ...request.context,
    compiler: nativePreparationContext(request.root).compiler,
  }
  const selection = { pipeline, context, identity: pipeline.loaders }
  const prepared = await compileMdx({ ...request, pipeline, context })
  const entry = { ...captureMdxCacheIdentity(selection), prepared, inputs: [] }
  rememberMdxCacheAdmission(entry, prepared, selection)
  expect(compatibleMdxCache(entry, selection)).toBe(true)
  await withNativeMdx({ ...request, pipeline }, async (native) => {
    expect((await native.compile()).source).toContain('original')
    await expect(native.compile({ renderer: 'removed' })).rejects.toThrow(
      'options.renderer',
    )
    await expect(
      native.compile({
        remarkPlugins: [
          () => () => {
            throw new Error('owned native plugin failure')
          },
        ],
      }),
    ).rejects.toThrow('page.mdx:Error: owned native plugin failure')
  })
  const module = join(directory, 'lib/index.js')
  // When
  writeFileSync(
    module,
    readFileSync(module, 'utf8') + '\n// changed installed compiler identity\n',
  )
  // Then
  expect(compatibleMdxCache(entry, selection)).toBe(false)
  expect(prepared.source).toContain('original')
})

it('misses a changed native options hash even when the same opaque Date identity remains live', async () => {
  // Given
  const date = new Date('2026-01-01T00:00:00.000Z')
  let constructions = 0
  const attacher = (options: { readonly date: Date }) => {
    constructions++
    return (tree: RemarkTree) => {
      const node = tree.children[0]?.children[0]
      if (node) node.value = String(options.date.getUTCFullYear())
    }
  }
  const request = fixture('@mdx-js/loader', {
    format: 'mdx',
    remarkPlugins: [[attacher, { date }]],
  })
  await withNativeMdx(request, async (native) => {
    await native.compile()
    const context = { ...request.context, compiler: native.compiler }
    const selection = {
      context,
      pipeline: request.pipeline,
      identity: request.pipeline.loaders,
    }
    const prepared = await compileMdx({ ...request, context })
    const entry = {
      ...captureMdxCacheIdentity(selection),
      prepared,
      inputs: [],
    }
    rememberMdxCacheAdmission(entry, prepared, selection)
    expect(compatibleMdxCache(entry, selection)).toBe(true)
    const count = constructions
    // When
    date.setUTCFullYear(2027)
    const changedNative = await native.compile()
    // Then
    expect(changedNative.source).toContain('2027')
    expect(constructions).toBe(count + 1)
    expect(compatibleMdxCache(entry, selection)).toBe(false)
    expect(prepared.source).toContain('2026')
  })
})

it('misses an effective option that becomes effectful without evaluating its new conversion', async () => {
  // Given
  const date = new Date(0)
  const attacher = () => () => undefined
  const request = fixture('@mdx-js/loader', {
    format: 'mdx',
    remarkPlugins: [[attacher, { date }]],
  })
  const prepared = await compileMdx(request)
  const selection = {
    pipeline: request.pipeline,
    context: request.context,
    identity: request.pipeline.loaders,
  }
  const entry = { ...captureMdxCacheIdentity(selection), prepared, inputs: [] }
  rememberMdxCacheAdmission(entry, prepared, selection)
  let effects = 0
  Object.defineProperty(date, 'toJSON', {
    value: () => {
      effects++
      return 'changed'
    },
  })
  // When
  const compatible = compatibleMdxCache(entry, selection)
  const error = await compileMdx(request).catch((cause: unknown) => cause)
  // Then
  expect(compatible).toBe(false)
  expect(error).toBeInstanceOf(MdxCompileError)
  expect(effects).toBe(0)
})

it('preserves an abnormal live-options boundary failure rather than reusing compiled bytes', async () => {
  // Given
  const token = Symbol('live options failure')
  let fail = false
  const leaf = new Proxy(
    { value: 1 },
    {
      has(target, key) {
        if (fail) throw token
        return Reflect.has(target, key)
      },
    },
  )
  const request = fixture('@mdx-js/loader', {
    format: 'mdx',
    remarkPlugins: [[() => () => undefined, { leaf }]],
  })
  const prepared = await compileMdx(request)
  const selection = {
    pipeline: request.pipeline,
    context: request.context,
    identity: request.pipeline.loaders,
  }
  const entry = { ...captureMdxCacheIdentity(selection), prepared, inputs: [] }
  rememberMdxCacheAdmission(entry, prepared, selection)
  // When
  fail = true
  const error = await Promise.resolve()
    .then(() => compatibleMdxCache(entry, selection))
    .catch((cause: unknown) => cause)
  // Then
  expect(error).toBe(token)
})
