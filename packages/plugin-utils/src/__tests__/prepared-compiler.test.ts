import { createRequire } from 'node:module'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

import {
  __setOxcParserForTest,
  buildStaticImportGraph,
  computeCompiledFiles,
} from '../import-graph'
import { createPreparedFixture } from './prepared-graph-fixture'

const projectRequire = createRequire(resolve('apps/landing/package.json'))
const compilerRequire = createRequire(projectRequire.resolve('@mdx-js/loader'))
const { compile } = compilerRequire('@mdx-js/mdx')
const { SourceMapGenerator, SourceMapConsumer } = compilerRequire('source-map')
let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it.each([false, true])(
  'scans real compiled MDX rather than fenced examples (async %s)',
  async (asyncHook) => {
    const markdown = [
      "import './real.js'",
      '',
      '# Heading',
      '',
      '```js',
      "import './missing'",
      "import { Box } from 'fake-package'",
      "export {Box} from './missing'",
      "require('./missing')",
      "import('./missing')",
      '```',
    ].join('\n')
    const entry = file('src/page.mdx', markdown)
    file('node_modules/fake-package/package.json', '{')
    const real = file('src/real.js')
    const compiled = await compile({ value: markdown, path: entry })
    const hook = (filename: string) =>
      filename === entry
        ? { code: String(compiled), map: compiled.map }
        : undefined
    const graph = await buildStaticImportGraph('src', undefined, {
      cwd: root,
      includeMdx: true,
      prepareSource: asyncHook ? async (filename) => hook(filename) : hook,
    })
    expect(graph.staticImports.get(entry)).toEqual(new Set([real]))
    expect(graph.dynamicImports.get(entry)).toEqual(new Set())
  },
)

it('rejects compiler errors with the actual Markdown filename and original cause', async () => {
  const entry = file('src/page.mdx', '<')
  let compilerCause: unknown
  const promise = buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
    prepareSource: async (filename) => {
      try {
        return String(await compile({ value: '<', path: filename }))
      } catch (cause) {
        compilerCause = cause
        throw cause
      }
    },
  })
  const error = await promise.catch((cause: unknown) => cause)
  expect(error).toBeInstanceOf(Error)
  if (!(error instanceof Error)) throw new Error('Expected compiler error')
  expect(error.message).toContain(entry)
  expect(error.cause).toBe(compilerCause)
})

it('selects and compiles a project-defined .mdown extension in scanner, graph and numbering', async () => {
  const markdown =
    "import './leaf'\n\n# Custom Markdown\n\n```js\nimport './missing'\n```"
  const entry = file('src/page.mdown', markdown)
  const leaf = file('src/leaf.mdown', '# Leaf')
  const graph = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: ['.mdown'],
    prepareSource: async (filename) => {
      return String(
        await compile(
          { value: filename === entry ? markdown : '# Leaf', path: filename },
          { format: 'mdx' },
        ),
      )
    },
  })
  expect(graph.staticImports.get(entry)).toEqual(new Set([leaf]))
  expect(computeCompiledFiles({ cwd: root, srcDir: 'src', graph })).toEqual([
    'src/leaf.mdown',
    'src/page.mdown',
  ])
})

it('follows a real compiler-injected project-root provider and prepares included dependencies once', async () => {
  const entry = file('src/page.mdx', '# Heading')
  const provider = file('mdx-components.js', "import 'aliased-library'")
  const remark = file('remark-extra.js')
  const rehype = file('rehype-extra.js')
  file('node_modules/actual/package.json', '{"main":"index.js"}')
  const library = file('node_modules/actual/index.js', "import './leaf.config'")
  const leaf = file('node_modules/actual/leaf.config.mjs')
  const visits: string[] = []
  const graph = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
    include: ['actual'],
    alias: {
      provider,
      'aliased-library': 'actual',
      'remark-extra': remark,
      'rehype-extra': rehype,
    },
    prepareSource: async (filename) => {
      visits.push(filename)
      if (filename !== entry) return undefined
      const inject =
        (specifier: string) => () => (tree: { children: unknown[] }) => {
          tree.children.unshift({
            type: 'mdxjsEsm',
            value: `import '${specifier}'`,
            data: {
              estree: {
                type: 'Program',
                sourceType: 'module',
                body: [
                  {
                    type: 'ImportDeclaration',
                    specifiers: [],
                    source: { type: 'Literal', value: specifier },
                  },
                ],
              },
            },
          })
        }
      const compiled = await compile(
        { value: '# Heading', path: filename },
        {
          providerImportSource: 'provider',
          SourceMapGenerator,
          remarkPlugins: [inject('remark-extra')],
          rehypePlugins: [inject('rehype-extra')],
        },
      )
      return { code: String(compiled), map: compiled.map }
    },
  })
  expect(graph.files).toEqual(
    [entry, provider, library, leaf, remark, rehype].sort(),
  )
  expect(visits.sort()).toEqual(graph.files)
})

it('remaps available-parser diagnostic positions through a real MDX compiler map', async () => {
  const entry = file('src/page.mdx', '# Heading')
  const compiled = await compile(
    { value: '# Heading', path: entry },
    { SourceMapGenerator },
  )
  expect(compiled.map).toBeDefined()
  const positions: {
    line: number
    column: number
    originalLine: number
    originalColumn: number
  }[] = []
  await SourceMapConsumer.with(
    compiled.map,
    null,
    (consumer: {
      eachMapping: (
        callback: (mapping: {
          generatedLine: number
          generatedColumn: number
          originalLine: number
          originalColumn: number
        }) => void,
      ) => void
    }) => {
      consumer.eachMapping((mapping) => {
        if (mapping.originalLine)
          positions.push({
            line: mapping.generatedLine,
            column: mapping.generatedColumn + 1,
            originalLine: mapping.originalLine,
            originalColumn: mapping.originalColumn + 1,
          })
      })
    },
  )
  const position = positions[0]
  if (!position) throw new Error('Real compiler map has no original mappings')
  __setOxcParserForTest({
    parseSync: () => ({
      errors: [
        {
          message: 'available parser fixture',
          line: position.line,
          column: position.column,
        },
      ],
    }),
  })
  await expect(
    buildStaticImportGraph('src', undefined, {
      cwd: root,
      includeMdx: true,
      prepareSource: () => ({ code: String(compiled), map: compiled.map }),
    }),
  ).rejects.toThrow(
    `${entry}:${position.originalLine}:${position.originalColumn}`,
  )
})
