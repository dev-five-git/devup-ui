import { createRequire } from 'node:module'
import { relative, resolve } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import * as wasm from '../../../../bindings/devup-ui-wasm/pkg'
import {
  buildStaticImportGraph,
  createModuleResolver,
  type PreparedSource,
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
let debug: boolean
beforeEach(() => {
  debug = wasm.isDebug()
  wasm.resetBuildState()
  wasm.setDebug(false)
})
afterEach(() => {
  wasm.setModuleResolver(undefined)
  wasm.resetBuildState()
  wasm.registerTheme({})
  wasm.setDebug(debug)
})

function extract(filename: string, source: string) {
  const output = wasm.codeExtract(
    filename,
    source,
    '@devup-ui/react',
    'df',
    true,
    false,
    false,
    {},
  )
  try {
    return { code: output.code, css: wasm.getCss(undefined, false) }
  } finally {
    output.free()
  }
}

it.each(['absolute', 'relative'])(
  'uses actual compiled MDX constants like TS constants (%s ids)',
  async (mode) => {
    const markdown = "export const PRIMARY = 'tomato'\n\n# Heading"
    const module = file('src/value.mdx', markdown)
    file('src/plain.ts', "export const PRIMARY = 'tomato'")
    const entry = file(
      'src/main.tsx',
      "import {PRIMARY} from './value.mdx'; import {Box} from '@devup-ui/react'; export const view = <Box color={PRIMARY}/>;",
    )
    const compiled = await compile(
      { value: markdown, path: module },
      { SourceMapGenerator },
    )
    const cache = new Map<string, PreparedSource>([
      [module, { code: String(compiled), map: compiled.map }],
    ])
    const prepareSource = (filename: string) => cache.get(filename)
    const toId = (filename: string) =>
      (mode === 'absolute' ? filename : relative(root, filename)).replaceAll(
        '\\',
        '/',
      )
    const resolver = createModuleResolver({ cwd: root, prepareSource, toId })
    const graph = await buildStaticImportGraph('src', undefined, {
      cwd: root,
      includeMdx: true,
      prepareSource,
    })
    expect(graph.staticImports.get(entry)).toEqual(new Set([module]))
    expect(resolver('./value.mdx', toId(entry))?.code).toBe(String(compiled))
    wasm.setModuleResolver(resolver)
    const prepared = extract(
      toId(entry),
      "import {PRIMARY} from './value.mdx'; import {Box} from '@devup-ui/react'; export const view = <Box color={PRIMARY}/>;",
    )
    wasm.resetBuildState()
    wasm.setModuleResolver(resolver)
    const ordinary = extract(
      toId(entry),
      "import {PRIMARY} from './plain'; import {Box} from '@devup-ui/react'; export const view = <Box color={PRIMARY}/>;",
    )
    expect(prepared.css).toBe(ordinary.css)
    expect(prepared.css).toContain('color:tomato')
    expect(prepared.code).not.toContain('--')
  },
)

it.each(['raw', 'promise'])(
  'rejects %s imported Markdown through the actual WASM callback',
  (mode) => {
    const module = file(
      'src/value.mdx',
      "export const PRIMARY = 'tomato'\n\n# Heading",
    )
    const entry = file('src/main.tsx')
    const resolver = createModuleResolver({
      cwd: root,
      prepareSource: () =>
        mode === 'promise' ? Promise.resolve('') : undefined,
    })
    wasm.setModuleResolver(resolver)
    try {
      extract(
        entry,
        "import {PRIMARY} from './value.mdx'; import {Box} from '@devup-ui/react'; export const view = <Box color={PRIMARY}/>;",
      )
    } catch (cause) {
      const error = resolver.remapError(cause)
      expect(error.message).toContain(`${entry}:1:1`)
      expect(error.message).toContain(module)
      expect(error.cause).toBe(cause)
      return
    }
    throw new Error('Expected raw/preparation rejection')
  },
)

it('leaves the real WASM importing-site error untouched after prepared-module evaluation', async () => {
  const markdown = "export const bad = () => 'red'\n\n# Heading"
  const module = file('src/value.mdx', markdown)
  const entry = file('src/main.tsx')
  const compiled = await compile(
    { value: markdown, path: module },
    { SourceMapGenerator },
  )
  const resolver = createModuleResolver({
    cwd: root,
    prepareSource: (filename) =>
      filename === module
        ? { code: String(compiled), map: compiled.map }
        : undefined,
  })
  wasm.setModuleResolver(resolver)
  try {
    extract(
      entry,
      "import {bad} from './value.mdx'; import {css} from '@devup-ui/react'; export const style = css({color:bad});",
    )
  } catch (cause) {
    const error = resolver.remapError(cause)
    const original = cause instanceof Error ? cause.message : String(cause)
    expect(error.message).toBe(original)
    expect(error.cause).toBe(cause)
    expect(resolver.remapError(error)).toBe(error)
    expect(error.message).toContain(`${entry}:1:`)
    return
  }
  throw new Error('Expected imported extraction failure')
})

it('remaps synthetic extractor-format diagnostics through a real MDX compiler map', async () => {
  const markdown = "export const PRIMARY = 'tomato'\n\n# Heading"
  const module = file('src/value.mdx', markdown)
  const compiled = await compile(
    { value: markdown, path: module },
    { SourceMapGenerator },
  )
  const resolver = createModuleResolver({
    cwd: root,
    toId: () => 'src/value.mdx',
    prepareSource: () => ({ code: String(compiled), map: compiled.map }),
  })
  resolver(module, 'main.ts')
  await SourceMapConsumer.with(
    compiled.map,
    null,
    (consumer: {
      eachMapping(
        callback: (mapping: {
          generatedLine: number
          generatedColumn: number
          originalLine: number
          originalColumn: number
          source: string
        }) => void,
      ): void
    }) => {
      const positions: string[] = []
      consumer.eachMapping((mapping) => {
        if (mapping.originalLine) {
          const error = resolver.remapError(
            `src/value.mdx:${mapping.generatedLine}:${mapping.generatedColumn + 1}: diagnostic`,
          )
          expect(error.message).toBe(
            `${module}:${mapping.originalLine}:${mapping.originalColumn + 1}: diagnostic`,
          )
          positions.push(error.message)
        }
      })
      expect(positions.length).toBeGreaterThan(0)
    },
  )
})
