import { expect, it } from 'bun:test'

import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'

it.each(
  (['webpack', 'turbo'] as const).flatMap((bundler) =>
    (['accessor', 'json'] as const).map((form) => ({ bundler, form })),
  ),
)(
  'refuses effectful compiler $form before speculative evaluation for $bundler',
  async ({ bundler, form }) => {
    // Given
    const request = fixture('@mdx-js/loader')
    let effects = 0
    const options = Object.defineProperty({}, 'custom', {
      enumerable: true,
      get() {
        effects++
        return 'value'
      },
    })
    const withJson = {
      toJSON() {
        effects++
        return {}
      },
    }
    const configured = { accessor: options, json: withJson }[form]
    // When
    const error = await compileMdx({
      ...request,
      pipeline: {
        ...request.pipeline,
        bundler,
        loaders: request.pipeline.loaders.map((loader) => ({
          ...loader,
          options: configured,
        })),
      },
    }).catch((cause: unknown) => cause)
    // Then
    expect(error).toBeInstanceOf(MdxCompileError)
    expect(effects).toBe(0)
  },
)

it('refuses missing native Compiler evidence rather than guessing a generation or app owner', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const context = {
    owner: {},
    generation: {},
    mode: request.context.mode,
    sourceMap: request.context.sourceMap,
  }
  // When
  const error = await compileMdx({ ...request, context }).catch(
    (cause: unknown) => cause,
  )
  // Then
  expect(error).toMatchObject({
    filename: request.filename,
    cause: { option: '_compiler' },
  })
})

it.each(['toJSON', 'valueOf', 'toString', 'toISOString', Symbol.toPrimitive])(
  'refuses an overridden Date conversion %s without running it',
  async (key) => {
    // Given
    const request = fixture('@mdx-js/loader')
    let effects = 0
    const date = Object.defineProperty(new Date(0), key, {
      value: () => {
        effects++
        return 0
      },
    })
    // When
    const error = await compileMdx({
      ...request,
      pipeline: {
        ...request.pipeline,
        loaders: request.pipeline.loaders.map((loader) => ({
          ...loader,
          options: { custom: date },
        })),
      },
    }).catch((cause: unknown) => cause)
    // Then
    expect(error).toBeInstanceOf(MdxCompileError)
    expect(effects).toBe(0)
  },
)
