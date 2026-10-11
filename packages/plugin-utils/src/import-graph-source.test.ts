import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, relative } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  computeFileReach,
  createModuleResolver,
} from './import-graph'

describe('vanilla-extract source graph resolution', () => {
  let cwd: string
  let srcDir: string

  beforeEach(() => {
    cwd = mkdtempSync(join(tmpdir(), 'devup-ui-ve-source-'))
    srcDir = join(cwd, 'src')
    mkdirSync(srcDir)
    writeFixture(
      'tsconfig.json',
      JSON.stringify({
        compilerOptions: { baseUrl: '.', paths: { '@/*': ['src/*'] } },
      }),
    )
  })

  afterEach(() => {
    rmSync(cwd, { recursive: true, force: true })
  })

  function writeFixture(path: string, code: string): void {
    const file = join(cwd, path)
    mkdirSync(dirname(file), { recursive: true })
    writeFileSync(file, code)
  }

  for (const extension of ['ts', 'js']) {
    for (const specifier of ['./theme.css', '@/theme.css']) {
      it(`uses the evaluator producer identity when ${specifier} resolves to .css.${extension}`, () => {
        // Given
        const producer = `src/theme.css.${extension}`
        const code = 'export const vars = { color: "var(--producer-color)" }'
        writeFixture('src/App.tsx', `import { vars } from '${specifier}'`)
        writeFixture(producer, code)
        const tsconfigPath = join(cwd, 'tsconfig.json')
        const toId = (path: string) => relative(cwd, path).replaceAll('\\', '/')
        const resolveModule = createModuleResolver({ cwd, tsconfigPath, toId })

        // When
        const graph = buildStaticImportGraph(srcDir, tsconfigPath)
        const evaluated = resolveModule(specifier, 'src/App.tsx')

        // Then: owner path/code and the canonical bucket use the same names.
        expect(evaluated).toEqual({ path: producer, code })
        expect([
          ...(graph.staticImports.get(join(srcDir, 'App.tsx')) ?? []),
        ]).toEqual([join(cwd, producer)])
        expect([
          ...(graph.staticImporters.get(join(cwd, producer)) ?? []),
        ]).toEqual([join(srcDir, 'App.tsx')])
        expect(buildCanonicalMap({ cwd, srcDir, graph })).toEqual({
          [producer]: 'src/App.tsx',
        })
        expect(computeFileReach({ cwd, srcDir, graph })).toEqual({
          'src/App.tsx': [0],
          [producer]: [0],
        })
      })
    }
  }

  it('keeps evaluator suffix precedence when raw CSS and both script producers coexist', () => {
    // Given: raw CSS is not an evaluator module, even when it exists exactly.
    writeFixture('src/App.tsx', "import './theme.css'")
    writeFixture('src/theme.css', 'body { color: red }')
    writeFixture('src/theme.css.ts', 'export const theme = "typescript"')
    writeFixture('src/theme.css.js', 'export const theme = "javascript"')

    // When
    const graph = buildStaticImportGraph(srcDir)
    const evaluated = createModuleResolver({ cwd })(
      './theme.css',
      'src/App.tsx',
    )

    // Then
    expect(evaluated).toEqual({
      path: join(srcDir, 'theme.css.ts'),
      code: 'export const theme = "typescript"',
    })
    expect([
      ...(graph.staticImports.get(join(srcDir, 'App.tsx')) ?? []),
    ]).toEqual([join(srcDir, 'theme.css.ts')])
    expect(buildCanonicalMap({ cwd, srcDir, graph })).toEqual({
      'src/theme.css.ts': 'src/App.tsx',
    })
  })

  it('keeps raw CSS and missing imports unresolved without weakening explicit extension guards', () => {
    // Given
    const specifiers = [
      './raw.css',
      './missing.css',
      './missing.ts',
      './tokens.js',
    ]
    writeFixture(
      'src/App.tsx',
      specifiers.map((value) => `import '${value}'`).join('\n'),
    )
    writeFixture('src/raw.css', 'body {}')
    writeFixture('src/tokens.js.ts', 'export const token = "wrong extension"')

    // When
    const graph = buildStaticImportGraph(srcDir)
    const resolveModule = createModuleResolver({ cwd })

    // Then
    expect(
      specifiers.map((value) => resolveModule(value, 'src/App.tsx')),
    ).toEqual([undefined, undefined, undefined, undefined])
    expect([
      ...(graph.staticImports.get(join(srcDir, 'App.tsx')) ?? []),
    ]).toEqual([])
    expect(buildCanonicalMap({ cwd, srcDir, graph })).toEqual({})
  })

  it('keeps resolved producers outside the scanned source set out of the graph', () => {
    // Given
    writeFixture(
      'src/App.tsx',
      "import '../theme.css'\nimport './theme.test.css'",
    )
    writeFixture('theme.css.ts', 'export const theme = "outside"')
    writeFixture('src/theme.test.css.ts', 'export const theme = "inside"')
    writeFixture('src/ignored.test.ts', 'export const ignored = 1')
    writeFixture('src/legacy.cjs', 'module.exports = 1')
    writeFixture(
      'src/Other.tsx',
      "import './ignored.test.ts'\nimport './legacy.cjs'",
    )

    // When
    const graph = buildStaticImportGraph(srcDir)
    const resolveModule = createModuleResolver({ cwd })

    // Then
    expect(resolveModule('../theme.css', 'src/App.tsx')?.path).toBe(
      join(cwd, 'theme.css.ts'),
    )
    expect(resolveModule('./legacy.cjs', 'src/Other.tsx')?.path).toBe(
      join(srcDir, 'legacy.cjs'),
    )
    expect([
      ...(graph.staticImports.get(join(srcDir, 'App.tsx')) ?? []),
    ]).toEqual([join(srcDir, 'theme.test.css.ts')])
    expect([
      ...(graph.staticImports.get(join(srcDir, 'Other.tsx')) ?? []),
    ]).toEqual([])
    expect(buildCanonicalMap({ cwd, srcDir, graph })).toEqual({
      'src/theme.test.css.ts': 'src/App.tsx',
    })
  })
})
