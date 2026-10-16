import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { MdxContextRefusal } from '../mdx-context-admission'
import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { originalCompile } from './mdx-boundary-fixture.test'
import { fullFixture } from './mdx-own-slot-fixture'

const operations = [
  '() => this.resourcePath',
  '() => this.loaderIndex',
  '() => this.data',
  '() => this.getOptions()',
  'this.getOptions',
  '(() => { const add = this.addDependency; const path = this.resourcePath + ".late"; return () => add(path) })()',
  'this.async',
  '() => loaders.length',
  '() => slot.path',
  '() => "path" in slot',
  '() => Object.getOwnPropertyDescriptor(slot, "options")',
  '(() => { const map = loaders.map; return () => Reflect.apply(map, loaders, [value => value.path]) })()',
] as const

it.each(
  ['pitch', 'normal'].flatMap((phase) =>
    [...operations, 'this.callback'].map((operation) => ({ phase, operation })),
  ),
)(
  'discards caught retained $phase access $operation after the installed runner advances',
  async ({ phase, operation }) => {
    // Given
    let retained: (() => unknown) | undefined
    const errors: unknown[] = []
    const capture = `const loaders = this.loaders; const slot = loaders[this.loaderIndex]; this.getOptions().capture(${operation});`
    const probe = () => {
      if (!retained) throw new TypeError('missing retained native occurrence')
      try {
        retained()
      } catch (cause) {
        if (!(cause instanceof Error)) throw cause
        errors.push(cause)
      }
    }
    const request = fullFixture(
      phase === 'pitch'
        ? 'this.getOptions().probe(); return source'
        : `${capture} return source`,
      phase === 'pitch' ? capture : '',
    )
    const [own, compiler, raw] = request.invocation.loaders
    const options = {
      capture: (value: () => unknown) => {
        retained = value
      },
      probe,
    }
    const post = join(request.root, 'retained-post.cjs')
    writeFileSync(
      post,
      'module.exports = function(source, map) { this.getOptions().probe(); this.callback(null, source, map) }',
    )
    const invocation =
      phase === 'pitch'
        ? {
            ...request.invocation,
            loaders: [own, compiler, { ...raw, options }],
          }
        : {
            ...request.invocation,
            compilerIndex: 2,
            loaders: [
              own,
              { loader: post, options },
              compiler,
              { ...raw, options },
            ],
          }
    // When
    const result = await compileMdx({ ...request, invocation }).catch(
      (cause: unknown) => cause,
    )
    // Then
    expect(result).toBeInstanceOf(MdxCompileError)
    expect(errors).toHaveLength(1)
    expect(result).toMatchObject({
      filename: request.filename,
      cause: { loader: raw.loader, index: phase === 'pitch' ? 2 : 3, phase },
    })
    if (operation.includes('value => value.path')) {
      const output = await originalCompile({
        ...request,
        pipeline: { ...request.pipeline, loaders: invocation.loaders.slice(1) },
      })
      expect(output.source).toContain('original')
      expect(retained?.()).toEqual(
        invocation.loaders.slice(1).map((loader) => loader.loader),
      )
    }
  },
)

it.each([...operations])(
  'closes retained access %s after successful native-runner completion',
  async (operation) => {
    // Given
    let retained: (() => unknown) | undefined
    const request = fullFixture(
      `const loaders = this.loaders; const slot = loaders[this.loaderIndex]; this.getOptions().capture(${operation}); return source`,
    )
    const [own, compiler, raw] = request.invocation.loaders
    const output = await compileMdx({
      ...request,
      invocation: {
        ...request.invocation,
        loaders: [
          own,
          compiler,
          {
            ...raw,
            options: {
              capture: (value: () => unknown) => {
                retained = value
              },
            },
          },
        ],
      },
    })
    const before = JSON.stringify(output)
    // When
    const error = (() => {
      try {
        return retained?.()
      } catch (cause) {
        if (!(cause instanceof Error)) throw cause
        return cause
      }
    })()
    // Then
    expect(error).toBeInstanceOf(MdxContextRefusal)
    expect(error).toMatchObject({
      loader: raw.loader,
      index: 2,
      phase: 'normal',
    })
    expect(JSON.stringify(output)).toBe(before)
    if (operation.includes('value => value.path')) {
      const native = await originalCompile({
        ...request,
        pipeline: {
          ...request.pipeline,
          loaders: [
            compiler,
            {
              ...raw,
              options: {
                capture: (value: () => unknown) => {
                  retained = value
                },
              },
            },
          ],
        },
      })
      expect(native.source).toContain('original')
      expect(retained?.()).toEqual([compiler.loader, raw.loader])
    }
  },
)

it('ignores a retained late callback without invoking another occurrence or changing accepted output', async () => {
  // Given
  let retained: (() => unknown) | undefined
  const request = fullFixture(
    'this.getOptions().capture(this.callback); return source',
  )
  const [own, compiler, raw] = request.invocation.loaders
  const output = await compileMdx({
    ...request,
    invocation: {
      ...request.invocation,
      loaders: [
        own,
        compiler,
        {
          ...raw,
          options: {
            capture: (value: () => unknown) => {
              retained = value
            },
          },
        },
      ],
    },
  })
  // When
  const late = (() => {
    try {
      return retained?.()
    } catch (cause) {
      if (!(cause instanceof Error)) throw cause
      return cause
    }
  })()
  // Then
  expect(late).toBeUndefined()
  expect(output.source).toContain('original')
})

it('preserves the exact first refusal when later occurrence retention also fails', async () => {
  // Given
  let original: unknown
  let retained: (() => unknown) | undefined
  const request = fullFixture(
    'const options = this.getOptions(); options.capture(() => this.resourcePath); try { void this._compiler } catch (cause) { if (!(cause instanceof Error)) throw cause; options.first(cause) }; return source',
  )
  const [own, compiler, raw] = request.invocation.loaders
  const post = join(request.root, 'after-first.cjs')
  writeFileSync(
    post,
    'module.exports = function(source) { this.getOptions().probe(); return source }',
  )
  const seen: unknown[] = []
  const options = {
    capture: (value: () => unknown) => {
      retained = value
    },
    first: (value: unknown) => {
      original = value
    },
    probe: () => {
      if (!retained) throw new TypeError('missing retained view')
      try {
        retained()
      } catch (cause) {
        if (!(cause instanceof Error)) throw cause
        seen.push(cause)
      }
    },
  }
  // When
  const result = await compileMdx({
    ...request,
    invocation: {
      ...request.invocation,
      compilerIndex: 2,
      loaders: [own, { loader: post, options }, compiler, { ...raw, options }],
    },
  }).catch((cause: unknown) => cause)
  // Then
  expect(result).toBeInstanceOf(MdxCompileError)
  if (!(result instanceof MdxCompileError))
    throw new TypeError('refused invocation escaped')
  expect(result.cause).toBe(original)
  expect(seen).toEqual([original])
  expect(result.cause).toMatchObject({
    property: '_compiler',
    loader: raw.loader,
    index: 3,
    phase: 'normal',
  })
})
