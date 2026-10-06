import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fullFixture } from './mdx-own-slot-fixture'

it.each([
  {
    body: 'if (!this._compilation) throw new Error("missing native _compilation"); return source',
    pitch: '',
  },
  { body: 'throw new Error("MDX invocation-private capture stop")', pitch: '' },
])(
  'publishes nothing when an actual raw loader bypasses or rejects ($body)',
  async ({ body, pitch }) => {
    // Given
    const request = fullFixture(body, pitch)
    // When / Then
    await expect(compileMdx(request)).rejects.toBeInstanceOf(MdxCompileError)
  },
)

it('rejects compiler bypass when an intermediate pitch reaches the own slot without compilation', async () => {
  // Given
  const request = fullFixture('return source', 'return "# pitch bypass"')
  const [own, compiler, raw] = request.invocation.loaders
  if (!own || !compiler || !raw) throw new TypeError('missing original slots')
  const invocation = {
    ...request.invocation,
    compilerIndex: 2,
    loaders: [own, raw, compiler],
  }
  // When / Then
  await expect(compileMdx({ ...request, invocation })).rejects.toMatchObject({
    cause: { fact: 'exactly one compiler before Devup normal capture' },
  })
})

it('rejects at the original runner callback when a downstream pitch bypasses Devup capture', async () => {
  // Given
  const request = fullFixture('return source')
  const downstream = join(request.root, 'downstream.cjs')
  writeFileSync(
    downstream,
    'module.exports = function(source) { return source }; module.exports.pitch = function() { return "bypass" }',
  )
  const invocation = {
    ...request.invocation,
    loaders: [downstream, ...request.invocation.loaders],
    ownIndex: 1,
    compilerIndex: 2,
  }
  // When / Then
  await expect(compileMdx({ ...request, invocation })).rejects.toMatchObject({
    filename: request.filename,
    cause: {
      fact: 'private-stop runner callback after Devup capture (pitch bypass)',
    },
  })
})

it.each([
  { ownIndex: -1, compilerIndex: 1 },
  { ownIndex: 1, compilerIndex: 0 },
  { ownIndex: 0, compilerIndex: 0.5 },
  { ownIndex: 0, compilerIndex: 99 },
])(
  'rejects missing actual index facts for $ownIndex / $compilerIndex',
  async (indices) => {
    // Given
    const request = fullFixture('return source')
    // When / Then
    await expect(
      compileMdx({
        ...request,
        invocation: { ...request.invocation, ...indices },
      }),
    ).rejects.toMatchObject({
      cause: { fact: 'original compiler/own slot facts' },
    })
  },
)

it.each(['raw-unresolved', 'second-compiler'])(
  'rejects incomplete original loader facts for %s',
  async (extra) => {
    // Given
    const request = fullFixture('return source')
    const compiler = request.invocation.loaders[1]
    if (!compiler) throw new TypeError('missing installed compiler')
    const invocation = {
      ...request.invocation,
      loaders: [
        ...request.invocation.loaders,
        extra === 'raw-unresolved' ? 'raw' : compiler,
      ],
    }
    // When / Then
    await expect(compileMdx({ ...request, invocation })).rejects.toBeInstanceOf(
      MdxCompileError,
    )
  },
)

it('refuses direct calls into the own normal when the runner has not reached its original slot', async () => {
  // Given
  const request = fullFixture('this.loaders[0].normal.call(this, source, map)')
  const [own, compiler, raw] = request.invocation.loaders
  if (!own || !compiler || !raw) throw new TypeError('missing original slots')
  // When / Then
  await expect(
    compileMdx({
      ...request,
      invocation: {
        ...request.invocation,
        compilerIndex: 2,
        loaders: [own, raw, compiler],
      },
    }),
  ).rejects.toMatchObject({
    cause: { fact: 'original runner normal position 0' },
  })
})

it.each(['.md', '.mdown'])(
  'keeps original %s resources and JSX output without renaming',
  async (extension) => {
    // Given
    const request = fullFixture('return source')
    const filename = join(request.root, 'unchanged' + extension)
    writeFileSync(filename, '# original')
    // When
    const output = await compileMdx({
      ...request,
      filename,
      invocation: { ...request.invocation, resource: filename },
    })
    // Then
    expect(output.filename).toBe(filename)
    expect(output.source).toContain('<_components.h1>')
  },
)

it('rejects a full invocation whose actual resource belongs to another source', async () => {
  // Given
  const request = fullFixture('return source')
  const invocation = {
    ...request.invocation,
    resource: request.filename + '.foreign?original=query#fragment',
  }
  // When / Then
  await expect(compileMdx({ ...request, invocation })).rejects.toMatchObject({
    cause: { fact: 'original resource path for the prepared source' },
  })
})

it.each([
  'this.loaderIndex = 2; this.loaders[2].normal.call(this, source, map)',
  'this.loaderIndex = 0; this.loaders[0].normalExecuted = true; try { this.loaders[0].normal.call(this, source, map) } catch (error) { throw new Error(error.message) }',
  'this.loaderIndex = 0; this.loaders[0].normalExecuted = true; try { this.loaders[0].normal.call(this, source, map) } catch (error) { if (!(error instanceof Error)) throw error } this.loaders[0].normal.call(this, source, map)',
])(
  'rejects extra executions or a forged stop callback even when a raw loader manipulates execution flags (%s)',
  async (body) => {
    // Given
    const request = fullFixture(body)
    const [own, compiler, raw] = request.invocation.loaders
    if (!own || !compiler || !raw) throw new TypeError('missing original slots')
    // When / Then
    await expect(
      compileMdx({
        ...request,
        invocation: {
          ...request.invocation,
          compilerIndex: 2,
          loaders: [own, raw, compiler],
        },
      }),
    ).rejects.toBeInstanceOf(MdxCompileError)
  },
)
