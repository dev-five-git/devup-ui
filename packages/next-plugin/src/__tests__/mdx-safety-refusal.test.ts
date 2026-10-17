import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'

const probes = [
  { name: 'read', expression: 'this._compilation', property: '_compilation' },
  {
    name: 'presence',
    expression: '"_compilation" in this',
    property: '_compilation',
  },
  {
    name: 'descriptor',
    expression: 'Object.getOwnPropertyDescriptor(this, "_compilation")',
    property: '_compilation',
  },
  { name: 'keys', expression: 'Reflect.ownKeys(this)', property: 'ownKeys' },
  {
    name: 'prototype',
    expression: 'Object.getPrototypeOf(this)',
    property: 'getPrototypeOf',
  },
  {
    name: 'nested read',
    expression: 'this._compiler.options',
    property: '_compiler',
  },
  {
    name: 'nested presence',
    expression: '"options" in this._compiler',
    property: '_compiler',
  },
  {
    name: 'nested descriptor',
    expression: 'Object.getOwnPropertyDescriptor(this._compiler, "options")',
    property: '_compiler',
  },
] as const

it.each(
  ['@next/mdx/mdx-js-loader', '@mdx-js/loader'].flatMap((compiler) =>
    probes.map((probe) => ({ compiler, ...probe })),
  ),
)(
  'discards successful raw completion after caught $name with $compiler',
  async ({ compiler, expression, property }) => {
    // Given: the loader catches both probes, then successfully supplies valid MDX.
    const request = fixture(compiler)
    const raw = join(request.root, 'caught-probe.cjs')
    const completions: string[] = []
    writeFileSync(
      raw,
      `module.exports = function() {
        const observe = this.getOptions().observe;
        try { void (${expression}); } catch (error) {
          if (!(error instanceof Error)) throw error;
          observe('caught-first');
        }
        try { void this.w21atSecondUnsupported; } catch (error) {
          if (!(error instanceof Error)) throw error;
          observe('caught-second');
        }
        observe('completed');
        return '# valid after caught probe';
      }`,
    )
    const pipeline = {
      ...request.pipeline,
      loaders: [
        ...request.pipeline.loaders,
        {
          loader: raw,
          options: { observe: (value: string) => completions.push(value) },
        },
      ],
    }
    // When
    const result = await compileMdx({ ...request, pipeline }).catch(
      (error: unknown) => error,
    )
    // Then: no PreparedMdx escapes, even though the raw loader completed normally.
    expect(completions).toContain('completed')
    expect(result).toBeInstanceOf(MdxCompileError)
    if (!(result instanceof MdxCompileError))
      throw new TypeError('caught refusal escaped as prepared source')
    expect(completions).toEqual(['caught-first', 'caught-second', 'completed'])
    expect(result.filename).toBe(request.filename)
    expect(result.message).toContain(raw)
    expect(result.message).toContain(property)
    expect(result.cause).toBeInstanceOf(Error)
    expect(result.message).not.toContain('w21atSecondUnsupported')
  },
)

it('refuses cacheable(false) when the raw loader completes successfully', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const raw = join(request.root, 'uncacheable.cjs')
  writeFileSync(
    raw,
    'module.exports = function() { this.cacheable(false); return "# valid uncacheable output" }',
  )
  // When
  const result = await compileMdx({
    ...request,
    pipeline: {
      ...request.pipeline,
      loaders: [...request.pipeline.loaders, { loader: raw }],
    },
  }).catch((error: unknown) => error)
  // Then: located refusal is the permitted alternative to proven nonreuse.
  expect(result).toBeInstanceOf(MdxCompileError)
  if (!(result instanceof MdxCompileError))
    throw new TypeError('uncacheable output escaped as reusable preparation')
  expect(result.message).toContain(raw)
  expect(result.message).toContain('cacheable')
})
