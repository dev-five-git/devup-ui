import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import * as wasm from '../../../../bindings/devup-ui-wasm/pkg'
import { buildStaticImportGraph, createModuleResolver } from '../import-graph'
import { aliasFixture } from './alias-array-fixture'

const projectRequire = createRequire(resolve('apps/landing/package.json'))
const compilerRequire = createRequire(projectRequire.resolve('@mdx-js/loader'))
const { compile } = compilerRequire('@mdx-js/mdx')
let fixture: ReturnType<typeof aliasFixture>
let debug: boolean
beforeEach(() => {
  fixture = aliasFixture()
  debug = wasm.isDebug()
  wasm.resetBuildState()
  wasm.setDebug(false)
})
afterEach(() => {
  wasm.setModuleResolver(undefined)
  wasm.resetBuildState()
  wasm.registerTheme({})
  wasm.setDebug(debug)
  fixture.dispose()
})

it('prepares only the chosen real Markdown filename and extracts its imported constant', async () => {
  const { root, entry, file } = fixture
  const markdown = "export const PRIMARY = 'tomato'\n\n# Heading"
  const chosen = file('provider.mdx', markdown)
  const compiled = String(await compile({ value: markdown, path: chosen }))
  const alias = { provider: [join(root, 'missing.mdx'), chosen] }
  const calls: string[] = []
  const prepareSource = (filename: string) => {
    calls.push(filename)
    return filename === chosen ? compiled : undefined
  }
  const resolver = createModuleResolver({ cwd: root, alias, prepareSource })
  expect(resolver('provider', entry)).toEqual({ path: chosen, code: compiled })
  expect(calls).toEqual([chosen])
  const source =
    "import {PRIMARY} from 'provider'; import {Box} from '@devup-ui/react'; export const view = <Box color={PRIMARY}/>;"
  file('src/main.ts', source)
  calls.length = 0
  const graph = await buildStaticImportGraph('src', undefined, {
    cwd: root,
    alias,
    includeMdx: true,
    prepareSource,
  })
  expect(graph.staticImports.get(entry)).toEqual(new Set([chosen]))
  expect(calls).toEqual([entry, chosen])
  wasm.setModuleResolver(resolver)
  const output = wasm.codeExtract(
    entry.replace(/\.ts$/, '.tsx'),
    source,
    '@devup-ui/react',
    'df',
    true,
    false,
    false,
    {},
  )
  try {
    expect(wasm.getCss(undefined, false)).toContain('color:tomato')
    expect(output.code).not.toContain('--')
  } finally {
    output.free()
  }
})
