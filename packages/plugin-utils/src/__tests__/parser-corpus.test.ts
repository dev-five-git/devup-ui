import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { extname, join, relative, resolve } from 'node:path'

import { expect, it, mock } from 'bun:test'

import {
  __setOxcParserForTest,
  buildStaticImportGraph,
  listSourceFiles,
} from '../import-graph'
import { scanImports } from '../import-scanner'
import { importTypeFixtures } from './import-type-fixtures'
import { jsxBoundaryFixtures } from './jsx-boundary-fixtures'
import { oracleScannerFixtures } from './oracle-scanner-fixtures'

it('keeps all package, landing and real compiled docs edges parser-independent', async () => {
  // Given: actual repository sources and the landing compiler/provider configuration.
  const cwd = resolve(import.meta.dir, '../../../..')
  const projectRequire = createRequire(join(cwd, 'apps/landing/package.json'))
  const compilerRequire = createRequire(
    projectRequire.resolve('@mdx-js/loader'),
  )
  const {
    compile,
  }: {
    readonly compile: (
      input: { readonly path: string; readonly value: string | undefined },
      options: { readonly providerImportSource: string },
    ) => Promise<{ readonly value: string }>
  } = compilerRequire('@mdx-js/mdx')
  const roots = readdirSync(join(cwd, 'packages'), { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => join(cwd, 'packages', entry.name, 'src'))
  const landing = join(cwd, 'apps/landing/src')
  roots.push(landing)
  const exclude = [
    '__tests__',
    '__regression__',
    '__fixtures__',
    'dist',
    'pkg',
    'df',
    'generated',
  ]
  const selection = { includeMdx: ['.mdx'] }
  const files = [
    ...new Set(
      roots.flatMap((root) => listSourceFiles(root, exclude, selection)),
    ),
  ].sort()
  const code = new Map(
    files.map((filename) => [filename, readFileSync(filename, 'utf8')]),
  )
  const docs = files.filter((filename) => /\.mdx?$/.test(filename))
  for (const filename of docs) {
    const compiled = await compile(
      { path: filename, value: code.get(filename) },
      { providerImportSource: '@/mdx-components' },
    )
    code.set(filename, String(compiled))
  }
  const options = {
    cwd,
    exclude,
    ...selection,
    prepareSource: (filename: string) =>
      /\.mdx?$/.test(filename) ? code.get(filename) : undefined,
  }
  const programRead = mock(() => ({
    type: 'ImportDeclaration',
    source: { value: 'corpus-phantom' },
  }))
  const parseSync = mock((filename: string, source: string) => {
    expect(code.get(filename)).toBe(source)
    return {
      errors: [],
      get program() {
        return programRead()
      },
    }
  })
  try {
    // When: run the real graph/resolution path both without and with optional diagnostics.
    __setOxcParserForTest(false)
    const disabled = await buildStaticImportGraph(roots, undefined, options)
    __setOxcParserForTest({ parseSync })
    const injected = await buildStaticImportGraph(roots, undefined, options)
    // Then: compare every edge, not just aggregate counts; prove the optional path ran.
    expect(injected).toEqual(disabled)
    expect(injected.files).toEqual(files)
    expect(parseSync.mock.calls.map(([filename]) => filename).sort()).toEqual(
      files,
    )
    expect(programRead).not.toHaveBeenCalled()
    const packages = files.filter((filename) => !filename.startsWith(landing))
    const app = files.filter(
      (filename) => filename.startsWith(landing) && !docs.includes(filename),
    )
    expect(packages.length).toBeGreaterThan(0)
    expect(app.length).toBeGreaterThan(0)
    expect(docs.length).toBeGreaterThan(0)
    for (const filename of docs) {
      expect(disabled.externalImports?.get(filename)).toContain(
        'react/jsx-runtime',
      )
      expect(disabled.externalImports?.get(filename)).toContain(
        '@/mdx-components',
      )
    }
    const entries = files.map((filename) => ({
      file: relative(cwd, filename).replaceAll('\\', '/'),
      extension: extname(filename),
      static: [...(disabled.staticImports.get(filename) ?? [])]
        .map((target) => relative(cwd, target).replaceAll('\\', '/'))
        .sort(),
      dynamic: [...(disabled.dynamicImports.get(filename) ?? [])]
        .map((target) => relative(cwd, target).replaceAll('\\', '/'))
        .sort(),
      external: [...(disabled.externalImports?.get(filename) ?? [])].sort(),
    }))
    const artifact = {
      roots: roots.map((root) => relative(cwd, root).replaceAll('\\', '/')),
      exclude,
      filter:
        'JS/TS source extensions plus .mdx; test/spec files and node_modules omitted',
      compiler:
        '@mdx-js/mdx via landing @mdx-js/loader; providerImportSource=@/mdx-components; no remark/rehype plugins',
      counts: {
        packageSources: packages.length,
        landingSources: app.length,
        compiledDocs: docs.length,
        total: files.length,
        diagnosticCalls: parseSync.mock.calls.length,
        programReads: programRead.mock.calls.length,
        staticEdges: entries.reduce(
          (sum, entry) => sum + entry.static.length,
          0,
        ),
        dynamicEdges: entries.reduce(
          (sum, entry) => sum + entry.dynamic.length,
          0,
        ),
        externalEdges: entries.reduce(
          (sum, entry) => sum + entry.external.length,
          0,
        ),
      },
      parity: true,
      targeted: jsxBoundaryFixtures.map(({ code, jsx, edges }) => {
        const actual = scanImports(code, jsx).map(
          ({ kind, specifier }) => `${kind}:${specifier}`,
        )
        expect(actual).toEqual([...edges])
        return { code, jsx, expected: edges, actual }
      }),
      typeTargeted: importTypeFixtures.map(({ code, jsx, edges }) => {
        const actual = scanImports(code, jsx).map(
          ({ kind, specifier }) => `${kind}:${specifier}`,
        )
        expect(actual).toEqual([...edges])
        return { code, jsx, expected: edges, actual }
      }),
      oracleTargeted: oracleScannerFixtures.map(({ code, loader, edges }) => {
        const actual = scanImports(
          code,
          loader.endsWith('x'),
          loader.startsWith('t'),
        ).map(({ kind, specifier }) => `${kind}:${specifier}`)
        expect(actual).toEqual([...edges])
        return { code, loader, expected: edges, actual }
      }),
      entries,
    }
    if (process.env.DEVUP_SCANNER_CORPUS_ARTIFACT)
      writeFileSync(
        process.env.DEVUP_SCANNER_CORPUS_ARTIFACT,
        JSON.stringify(artifact, null, 2),
      )
  } finally {
    __setOxcParserForTest(undefined)
  }
}, 30000)
