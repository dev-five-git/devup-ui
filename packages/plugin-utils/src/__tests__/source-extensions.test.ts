import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  buildStaticImportGraph,
  createModuleResolver,
  listSourceFiles,
} from '../import-graph'
import {
  SOURCE_EXTENSIONS,
  SOURCE_FILE_REGEX,
  sourceFileCandidates,
} from '../source-extensions'

let root: string

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-ui-source-extensions-'))
})

afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

function write(files: Record<string, string>) {
  for (const [path, code] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), code)
  }
}

describe('SOURCE_FILE_REGEX', () => {
  it('matches every source extension and nothing else', () => {
    for (const extension of SOURCE_EXTENSIONS)
      expect(SOURCE_FILE_REGEX.test(`a/b${extension}`)).toBe(true)
    for (const path of ['a.css', 'a.json', 'a.mdx', 'a.d', 'a.mtsx', 'ts'])
      expect(SOURCE_FILE_REGEX.test(path)).toBe(false)
  })
})

describe('sourceFileCandidates', () => {
  it('adds the TypeScript files a JavaScript extension stands for', () => {
    expect(sourceFileCandidates('a.js')).toEqual(['a.js', 'a.ts', 'a.tsx'])
    expect(sourceFileCandidates('a.jsx')).toEqual(['a.jsx', 'a.tsx'])
    expect(sourceFileCandidates('a.mjs')).toEqual(['a.mjs', 'a.mts'])
    expect(sourceFileCandidates('a.cjs')).toEqual(['a.cjs', 'a.cts'])
    expect(sourceFileCandidates('a.mts')).toEqual(['a.mts'])
  })
})

describe('modern module extensions', () => {
  it('lists, parses and links ESM and CommonJS TypeScript modules', () => {
    write({
      'src/index.tsx':
        "import './esm.mjs'\nimport './cjs.cjs'\nimport './plain.js'\nimport './script.cjs'\n",
      'src/esm.mts': "import { b } from './b.js'\nexport const a = b\n",
      'src/cjs.cts': 'export const c = 1\n',
      'src/plain.ts': 'export const p = 1\n',
      'src/b.ts': 'export const b = 1\n',
      'src/script.cjs': 'module.exports = 1\n',
      'src/types.d.mts': 'export {}\n',
      'src/style.css': 'a {}\n',
    })
    const srcDir = join(root, 'src')
    expect(listSourceFiles(srcDir)).toEqual(
      [
        'b.ts',
        'cjs.cts',
        'esm.mts',
        'index.tsx',
        'plain.ts',
        'script.cjs',
        'types.d.mts',
      ].map((file) => join(srcDir, file)),
    )
    const graph = buildStaticImportGraph(srcDir)
    expect([
      ...(graph.staticImports.get(join(srcDir, 'index.tsx')) ?? []),
    ]).toEqual(
      ['esm.mts', 'cjs.cts', 'plain.ts', 'script.cjs'].map((file) =>
        join(srcDir, file),
      ),
    )
    expect([
      ...(graph.staticImports.get(join(srcDir, 'esm.mts')) ?? []),
    ]).toEqual([join(srcDir, 'b.ts')])
  })

  it('resolves the TypeScript source of a JavaScript import', () => {
    write({
      'src/tokens.mts': 'export const a = 1',
      'src/colors.cts': 'export const b = 2',
      'src/theme.ts': 'export const c = 3',
    })
    const resolveModule = createModuleResolver({ cwd: root })
    expect(resolveModule('./tokens.mjs', 'src/App.tsx')?.code).toBe(
      'export const a = 1',
    )
    expect(resolveModule('./colors.cjs', 'src/App.tsx')?.code).toBe(
      'export const b = 2',
    )
    expect(resolveModule('./theme.js', 'src/App.tsx')?.code).toBe(
      'export const c = 3',
    )
    expect(resolveModule('./tokens.mts', 'src/App.tsx')?.code).toBe(
      'export const a = 1',
    )
    expect(resolveModule('./missing.mjs', 'src/App.tsx')).toBeUndefined()
  })
})
