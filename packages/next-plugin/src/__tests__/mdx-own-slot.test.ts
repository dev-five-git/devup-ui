import { writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { compileMdx } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'

it('retains original compiler requests when a supplied segment observes descriptors', async () => {
  // Given: this is a supplied segment, not a full native invocation certificate.
  const request = fixture('@mdx-js/loader', { jsx: true })
  const compiler = request.pipeline.loaders[0]
  if (!compiler) throw new TypeError('missing compiler')
  const observations: unknown[] = []
  const raw = join(request.root, 'raw.cjs')
  writeFileSync(
    raw,
    `module.exports = function(source) {
    this.getOptions().observe(this.loaders.map(x => ({ path: x.path, query: x.query, ident: x.ident })))
    return source
  }`,
  )
  const pipeline = {
    ...request.pipeline,
    loaders: [
      compiler,
      {
        loader: raw,
        options: { observe: (value: unknown) => observations.push(value) },
        ident: 'caller-raw',
      },
    ],
  }
  // When
  const output = await compileMdx({ ...request, pipeline })
  // Then
  expect(output.source).toContain('original')
  expect(observations).toEqual([
    [
      { path: compiler.loader, query: '?{"jsx":true}', ident: undefined },
      { path: raw, query: '??caller-raw', ident: 'caller-raw' },
    ],
  ])
})

it('captures post-compiler bytes at the existing Devup slot when full descriptors are supplied', async () => {
  // Given
  const request = fixture('@mdx-js/loader', { jsx: true })
  const compiler = request.pipeline.loaders[0]
  if (!compiler) throw new TypeError('missing compiler')
  const raw = join(request.root, 'post.cjs')
  writeFileSync(
    raw,
    `module.exports = function(source, map) {
    const done = this.async(); this.addDependency(this.resourcePath + '.post')
    queueMicrotask(() => done(null, Buffer.from(source + '\\n// post-compiler'), map))
  }; module.exports.raw = true`,
  )
  const downstream = join(request.root, 'downstream.cjs')
  writeFileSync(
    downstream,
    `module.exports = require(${JSON.stringify(resolve(import.meta.dir, 'mdx-own-slot-normals.cjs'))}).downstream`,
  )
  const invocation = {
    resource: request.filename,
    loaders: [
      downstream,
      { loader: resolve(import.meta.dir, '../../dist/loader.cjs') },
      { loader: raw, fragment: '#post', type: 'commonjs' },
      compiler,
    ],
    ownIndex: 1,
    compilerIndex: 3,
  }
  // When
  const output = await compileMdx({ ...request, invocation })
  // Then
  expect(output.source).toEndWith('// post-compiler')
  expect(output.dependencies).toContain(request.filename + '.post')
  expect(output.map).toBeDefined()
})
