import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import { buildStaticImportGraph, listSourceFiles } from '../import-graph'
import {
  GRAPH_SOURCE_FILE_RE,
  MDX_FILE_RE,
  POST_COMPILED_MDX_RE,
  SOURCE_FILE_RE,
} from '../shared'

const parent = join(
  tmpdir(),
  'opencode',
  'workers',
  'w20-plugins-core',
  'mdx-shared',
)
let root: string
beforeEach(() => {
  mkdirSync(parent, { recursive: true })
  root = realpathSync(mkdtempSync(join(parent, 'exclude-')))
})
afterEach(() => rmSync(root, { recursive: true, force: true }))

it('skips named directories at every depth without reintroducing imported files', () => {
  const fixtures = {
    'src/z.ts':
      "import './generated/a'\nimport('./nested/generated/b')\nimport './node_modules/local/c'\nimport './nested/A.MDX'",
    'src/generated/a.ts': 'a',
    'src/nested/generated/b.ts': 'b',
    'src/node_modules/local/c.ts': 'c',
    'src/nested/A.MDX': "import '../value.CTS'\n\n# Heading",
    'src/value.CTS': 'value',
    'src/nested/ignored.TEST.TS': 'test',
    'app/generated/d.ts': 'd',
    'app/a.mjs': 'a',
  }
  for (const [path, code] of Object.entries(fixtures)) {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), code)
  }
  const graph = buildStaticImportGraph(['src', 'app'], undefined, {
    cwd: root,
    exclude: ['generated'],
  })
  expect(graph.files).toEqual(
    ['app/a.mjs', 'src/nested/A.MDX', 'src/value.CTS', 'src/z.ts'].map((path) =>
      join(root, path),
    ),
  )
  expect(graph.staticImports.get(join(root, 'src/nested/A.MDX'))).toEqual(
    new Set([join(root, 'src/value.CTS')]),
  )
  expect(listSourceFiles(join(root, 'src'), ['generated'])).toEqual(
    graph.files.slice(1),
  )
})

it.each(['.mts', '.cts', '.mjs', '.cjs', '.tsx', '.jsx'])(
  'matches uppercase modern %s extensions without treating compiled MDX as raw MDX',
  (extension) => {
    expect(SOURCE_FILE_RE.test(`file${extension.toUpperCase()}`)).toBe(true)
    expect(GRAPH_SOURCE_FILE_RE.test(`file${extension.toUpperCase()}`)).toBe(
      true,
    )
    expect(
      POST_COMPILED_MDX_RE.test(`file.MDX${extension.toUpperCase()}`),
    ).toBe(true)
    expect(MDX_FILE_RE.test(`file.MDX${extension}`)).toBe(false)
    expect(MDX_FILE_RE.test('file.MDX')).toBe(true)
  },
)
