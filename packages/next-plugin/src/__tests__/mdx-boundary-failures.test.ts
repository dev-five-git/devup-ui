import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'

const failures = [
  {
    name: 'cycle',
    options: () => {
      const cyclic: { self?: unknown } = {}
      cyclic.self = cyclic
      return { remarkPlugins: [[() => undefined, cyclic]] }
    },
    source: '# original',
    line: 1,
  },
  {
    name: 'processor construction',
    options: () => ({
      remarkPlugins: [
        () => {
          throw new Error('processor construction')
        },
      ],
    }),
    source: '# original',
    line: 1,
  },
  {
    name: 'throwing plugin',
    options: () => ({
      remarkPlugins: [
        () => () => {
          throw new Error('transformer failure')
        },
      ],
    }),
    source: '# original',
    line: 1,
  },
  {
    name: 'missing plugin',
    options: () => ({ remarkPlugins: ['missing-boundary-plugin'] }),
    source: '# original',
    line: 1,
  },
  { name: 'syntax', options: () => ({}), source: '# title\n\n<Box', line: 3 },
] as const
it.each(
  ['@next/mdx/mdx-js-loader', '@mdx-js/loader'].flatMap((compiler) =>
    failures.map((failure) => ({ compiler, ...failure })),
  ),
)(
  'locates $name with $compiler without unhandled rejections',
  async ({ compiler, options, source, line }) => {
    // Given
    const request = fixture(compiler, options())
    writeFileSync(request.filename, source)
    const unhandled: unknown[] = []
    const listener = (error: unknown) => {
      unhandled.push(error)
    }
    process.on('unhandledRejection', listener)
    try {
      // When
      const error = await compileMdx(request).catch((cause: unknown) => cause)
      // Then
      expect(error).toBeInstanceOf(MdxCompileError)
      if (!(error instanceof MdxCompileError))
        throw new TypeError('expected compile error')
      expect(error.filename).toBe(request.filename)
      expect(error.cause).toBeInstanceOf(Error)
      expect(error.message).toContain(`${request.filename}:${line}:`)
      await new Promise<void>((done) => setImmediate(done))
      expect(unhandled).toEqual([])
    } finally {
      process.off('unhandledRejection', listener)
    }
  },
)

it.each([
  { directory: '@next/mdx', entry: 'mdx-js-loader.js', version: '16.3.7' },
  { directory: '@next/mdx', entry: 'mdx-js-loader.js', version: '16.3.6' },
  { directory: '@mdx-js/loader', entry: 'index.cjs', version: '3.1.2' },
  { directory: '@mdx-js/loader', entry: 'index.cjs', version: '3.1.1' },
])(
  'keeps unknown wrappers original for $directory version $version',
  async ({ directory: relative, entry, version }) => {
    // Given
    const request = fixture('@mdx-js/loader')
    const directory = join(request.root, relative)
    mkdirSync(directory, { recursive: true })
    writeFileSync(join(directory, 'package.json'), JSON.stringify({ version }))
    const loader = join(directory, entry)
    writeFileSync(
      loader,
      'module.exports = function() { return "original unknown wrapper" }',
    )
    // When
    const output = await compileMdx({
      ...request,
      pipeline: { ...request.pipeline, loaders: [{ loader }] },
    })
    // Then
    expect(output.source).toBe('original unknown wrapper')
  },
)
