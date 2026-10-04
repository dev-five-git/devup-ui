import { mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import { composeMdxRules, requireMdxPipeline } from '../mdx-pipeline'
import { compileMdx, createMdxDeadline } from '../mdx-prepare'

const installedRoot = resolve(import.meta.dir, '../../../../apps/landing')
const installed = createRequire(join(installedRoot, 'package.json'))
const roots: string[] = []
afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})

function project(source = '# original') {
  const root = mkdtempSync(join(tmpdir(), 'devup-mdx-chain-'))
  roots.push(root)
  symlinkSync(
    join(installedRoot, 'node_modules'),
    join(root, 'node_modules'),
    'junction',
  )
  const filename = join(root, 'real page.mdx')
  writeFileSync(filename, source)
  return { root, filename }
}

it.each([true, false])(
  'uses the actual JS compiler options when jsx is %s',
  async (jsx) => {
    // Given
    const { root, filename } = project()
    const remark = join(root, 'remark.cjs')
    writeFileSync(
      remark,
      `module.exports = function(options) { return function(tree) { tree.children[0].children[0].value = options.text } }`,
    )
    const rehype = join(root, 'rehype.cjs')
    writeFileSync(
      rehype,
      `module.exports = function(options) { return function(tree) { tree.children[0].tagName = options.tag } }`,
    )
    const options = {
      jsx,
      remarkPlugins: [[remark, { text: 'configured plugin' }]],
      rehypePlugins: [[rehype, { tag: 'h2' }]],
      providerImportSource: 'custom-provider',
    }
    const result = composeMdxRules(
      {
        bundler: 'turbo',
        rules: {
          '*': {
            loaders: [
              { loader: installed.resolve('@next/mdx/mdx-js-loader'), options },
            ],
            as: '*.tsx',
          },
        },
        aliases: {},
      },
      { loader: 'devup' },
    )
    // When
    const output = await compileMdx({
      root,
      filename,
      pipeline: requireMdxPipeline(filename, result.pipelines[0]),
      signal: new AbortController().signal,
      deadline: createMdxDeadline(),
    })
    // Then
    expect(output.filename).toBe(filename)
    expect(output.source).toContain('configured plugin')
    expect(output.source).toContain('custom-provider')
    expect(output.source).toContain('h2')
    expect(output.source.includes('<')).toBe(jsx)
    expect(output.dependencies).toContain(filename)
    expect(output.dependencies).toContain(remark)
    expect(output.dependencies).toContain(rehype)
    expect(options.remarkPlugins[0]?.[0]).toBe(remark)
  },
)

it('runs only the pre-extraction segment when actual MDX has raw async loaders to its right', async () => {
  // Given
  const { root, filename } = project()
  const raw = join(root, 'raw.cjs')
  const downstream = join(root, 'downstream.cjs')
  writeFileSync(
    raw,
    `module.exports = function(source) { const done = this.async(); this.addDependency(this.resourcePath + '.raw'); queueMicrotask(() => done(null, '# raw async ' + this.resourcePath)) }`,
  )
  writeFileSync(
    downstream,
    `module.exports = function() { throw new Error('downstream must not run') }`,
  )
  const result = composeMdxRules(
    {
      bundler: 'webpack',
      rules: [
        {
          test: /\.mdx$/,
          use: [
            downstream,
            {
              loader: installed.resolve('@next/mdx/mdx-js-loader'),
              options: { jsx: true },
            },
            raw,
          ],
        },
      ],
      aliases: { provider: './provider.tsx' },
    },
    { loader: 'devup' },
  )
  // When
  const output = await compileMdx({
    root,
    filename,
    pipeline: requireMdxPipeline(filename, result.pipelines[0]),
    signal: new AbortController().signal,
    deadline: createMdxDeadline(),
  })
  // Then
  expect(output.source).toContain('raw async')
  expect(output.source).toContain('real page.mdx')
  expect(output.dependencies).toContain(`${filename}.raw`)
})

it.each(['@next/mdx/mdx-js-loader', '@mdx-js/loader'])(
  'preserves shared and function option references with %s',
  async (compiler) => {
    // Given
    const { root, filename } = project()
    const shared = { text: 'identity output' }
    const callback = () => shared.text
    const configured = { left: shared, right: shared, callback }
    const observations: unknown[] = []
    type Options = typeof configured
    const remark = (options: Options) => {
      observations.push(options, options.left, options.right, options.callback)
      return (tree: { children: { children: { value: string }[] }[] }) => {
        const text = tree.children[0]?.children[0]
        if (text) text.value = options.callback()
      }
    }
    const composed = composeMdxRules(
      {
        bundler: 'webpack',
        rules: [
          {
            use: [
              {
                loader: installed.resolve(compiler),
                options: { remarkPlugins: [[remark, configured]] },
              },
            ],
          },
        ],
        aliases: {},
      },
      { loader: 'devup' },
    )
    // When
    const output = await compileMdx({
      root,
      filename,
      pipeline: requireMdxPipeline(filename, composed.pipelines[0]),
      signal: new AbortController().signal,
      deadline: createMdxDeadline(),
    })
    // Then
    expect(observations).toEqual([configured, shared, shared, callback])
    expect(observations[0]).toBe(configured)
    expect(observations[1]).toBe(shared)
    expect(observations[2]).toBe(shared)
    expect(observations[3]).toBe(callback)
    expect(output.source).toContain('identity output')
  },
)

it('retains the real module and compiler location when installed MDX rejects syntax', async () => {
  // Given
  const { root, filename } = project('# hello\n\n<Box')
  const result = composeMdxRules(
    {
      bundler: 'turbo',
      rules: {
        '*': { loaders: [installed.resolve('@next/mdx/mdx-js-loader')] },
      },
      aliases: {},
    },
    { loader: 'devup' },
  )
  // When / Then
  await expect(
    compileMdx({
      root,
      filename,
      pipeline: requireMdxPipeline(filename, result.pipelines[0]),
      signal: new AbortController().signal,
      deadline: createMdxDeadline(),
    }),
  ).rejects.toMatchObject({ filename, line: 3 })
})

it.each([
  { remarkPlugins: ['missing-devup-mdx-plugin'] },
  { rehypePlugins: 'malformed' },
])(
  'observes installed Next plugin import failures when options are %j',
  async (options) => {
    // Given
    const { root, filename } = project()
    const result = composeMdxRules(
      {
        bundler: 'turbo',
        rules: {
          '*': {
            loaders: [
              { loader: installed.resolve('@next/mdx/mdx-js-loader'), options },
            ],
          },
        },
        aliases: {},
      },
      { loader: 'devup' },
    )
    // When / Then
    await expect(
      compileMdx({
        root,
        filename,
        pipeline: requireMdxPipeline(filename, result.pipelines[0]),
        signal: new AbortController().signal,
        deadline: createMdxDeadline(),
      }),
    ).rejects.toMatchObject({ filename })
  },
)
