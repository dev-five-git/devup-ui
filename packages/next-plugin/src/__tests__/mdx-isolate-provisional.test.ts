import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxIsolateOwner } from '../mdx-isolate-owner'
import { compileMdx } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'

async function provisionalFixture(raw: string) {
  const f = fixture('@next/mdx/mdx-js-loader')
  const input = join(f.root, 'map-input.cjs')
  writeFileSync(input, raw)
  const pipeline = {
    ...f.pipeline,
    bundler: 'turbo' as const,
    loaders: [...f.pipeline.loaders, { loader: input }],
  }
  const canonical = await compileMdx({ ...f, pipeline, context: f.context })
  const state = {
    sources: [canonical.source],
    watches: [...canonical.dependencies],
    finalized: true,
  }
  const owner = createMdxIsolateOwner(f.root)
  return {
    ...f,
    pipeline,
    canonical,
    state,
    owner,
    demand: { run: 'build', filename: f.filename, pipeline, timeoutMs: 5000 },
    binding: { run: 'build', filename: f.filename, canonical },
  }
}

const failing = `module.exports = function(source) { if (!this.sourceMap) { const error = new Error('getStaticProps-only false failure'); error.place = {line:7,column:4}; throw error } return source }`

it('leaves canonical state valid when getStaticProps-style overapproximation has an unused failing false outcome', async () => {
  // Given
  const f = await provisionalFixture(failing)
  const before = structuredClone(f.state)
  try {
    // When
    await f.owner.prepare(f.demand, f.signal)
    // Then
    expect(f.state).toEqual(before)
  } finally {
    await f.owner.dispose()
  }
})

it('refuses the later binding with its located first failure before any new publication', async () => {
  // Given
  const f = await provisionalFixture(failing)
  await f.owner.prepare(f.demand, f.signal)
  let publications = 0
  try {
    // When
    const error = await f.owner.bind(f.binding).then(
      (output) => {
        publications++
        f.state.sources.push(output.source)
      },
      (cause: unknown) => cause,
    )
    // Then
    expect(error).toMatchObject({
      failure: { filename: f.filename, line: 7, column: 4 },
    })
    expect(publications).toBe(0)
    expect(f.state.sources).toEqual([f.canonical.source])
  } finally {
    await f.owner.dispose()
  }
})

it('leaves canonical state unchanged when a byte-different false result is never requested', async () => {
  // Given
  const f = await provisionalFixture(
    `module.exports = function(source) { return this.sourceMap ? source : source.replace('original', 'different') }`,
  )
  const before = structuredClone(f.state)
  try {
    // When
    await f.owner.prepare(f.demand, f.signal)
    // Then
    expect(f.state).toEqual(before)
  } finally {
    await f.owner.dispose()
  }
})

it('checks byte agreement at explicit binding before allowing publication', async () => {
  // Given
  const f = await provisionalFixture(
    `module.exports = function(source) { return this.sourceMap ? source : source.replace('original', 'different') }`,
  )
  await f.owner.prepare(f.demand, f.signal)
  try {
    // When
    const failure = await f.owner.bind(f.binding).then(
      (output) => output,
      (cause: unknown) => cause,
    )
    // Then
    expect(failure).toBeInstanceOf(Error)
    expect(failure).toMatchObject({
      message: expect.stringContaining('true/false compiler bytes disagree'),
    })
  } finally {
    await f.owner.dispose()
  }
})

it('checks provisional input freshness at explicit binding', async () => {
  // Given
  const f = await provisionalFixture(
    'module.exports = function(source) { return source }',
  )
  await f.owner.prepare(f.demand, f.signal)
  writeFileSync(f.filename, '# changed')
  try {
    // When
    const failure = await f.owner.bind(f.binding).then(
      (output) => output,
      (cause: unknown) => cause,
    )
    // Then
    expect(failure).toBeInstanceOf(Error)
    expect(failure).toMatchObject({
      message: expect.stringContaining('provisional input changed'),
    })
  } finally {
    await f.owner.dispose()
  }
})
