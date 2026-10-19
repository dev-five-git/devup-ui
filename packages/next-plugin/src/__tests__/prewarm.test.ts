import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import type { StaticImportGraph } from '@devup-ui/plugin-utils'
import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import { collectProductionPrewarmFiles } from '../prewarm'

describe('collectProductionPrewarmFiles', () => {
  let cwd: string

  beforeEach(() => {
    cwd = mkdtempSync(join(tmpdir(), 'devup-ui-next-prewarm-'))
    writeFileSync(join(cwd, 'package.json'), '{"private":true}')
  })

  afterEach(() => {
    rmSync(cwd, { recursive: true, force: true })
  })

  function writePackage(
    name: string,
    exports: Record<string, unknown> | string,
    files: Record<string, string>,
  ): void {
    const packageDir = join(cwd, 'node_modules', ...name.split('/'))
    mkdirSync(packageDir, { recursive: true })
    writeFileSync(
      join(packageDir, 'package.json'),
      JSON.stringify({ name, exports }),
    )
    for (const [filename, contents] of Object.entries(files)) {
      const path = join(packageDir, filename)
      mkdirSync(dirname(path), { recursive: true })
      writeFileSync(path, contents)
    }
  }

  function makeGraph(files: string[], specifiers: string[]): StaticImportGraph {
    const source = files[0] ?? join(cwd, 'src/app/page.tsx')
    return {
      files,
      fileSet: new Set(files),
      staticImports: new Map(files.map((file) => [file, new Set()])),
      staticImporters: new Map(files.map((file) => [file, new Set()])),
      dynamicTargets: new Set(),
      dynamicImports: new Map(files.map((file) => [file, new Set()])),
      externalImports: new Map([[source, new Set(specifiers)]]),
    }
  }

  it('includes all source candidates and ESM entries accepted by the loader', () => {
    writePackage(
      '@devup-ui/reset-css',
      {
        '.': { import: './dist/index.mjs', require: './dist/index.cjs' },
      },
      { 'dist/index.cjs': '', 'dist/index.mjs': '' },
    )
    writePackage('@devup-editor/editor', './index.js', { 'index.js': '' })
    writePackage(
      '@acme/ui',
      { '.': { import: './index.mjs', require: './index.cjs' } },
      { 'index.cjs': '', 'index.mjs': '' },
    )
    writePackage('design-system', './index.js', { 'index.js': '' })
    writePackage('@devup-ui/cjs-only', './index.cjs', { 'index.cjs': '' })
    writePackage('@devup-ui/data', './data.json', { 'data.json': '{}' })

    const page = join(cwd, 'src/app/page.tsx')
    const templateTarget = join(cwd, 'src/demos/template-target.tsx')
    const files = collectProductionPrewarmFiles({
      cwd,
      graph: makeGraph(
        [page, templateTarget],
        [
          '',
          '@broken',
          '#internal',
          'node:fs',
          'react',
          '@devup-ui/reset-css',
          '@devup-editor/editor',
          '@acme/ui',
          'design-system',
          '@devup-ui/cjs-only',
          '@devup-ui/data',
          '@devup-ui/missing',
        ],
      ),
      expectedBaseFiles: ['src/app/page.tsx'],
      libPackage: '@acme/ui',
      include: ['design-system'],
    })

    expect(files).toEqual(
      [
        'node_modules/@acme/ui/index.mjs',
        'node_modules/@devup-editor/editor/index.js',
        'node_modules/@devup-ui/reset-css/dist/index.mjs',
        'node_modules/design-system/index.js',
        'src/app/page.tsx',
        'src/demos/template-target.tsx',
      ].sort(),
    )
  })

  it('normalizes an absolute expected file without source or package imports', () => {
    const page = join(cwd, 'src/app/page.tsx')

    expect(
      collectProductionPrewarmFiles({
        cwd,
        graph: makeGraph([], []),
        expectedBaseFiles: [page],
        libPackage: '@',
        include: [],
      }),
    ).toEqual(['src/app/page.tsx'])
  })

  it('follows accepted barrels, relative cycles, lazy modules and importer-local accepted dependencies', () => {
    // Given a package barrel with a nested dependency and a rejected sibling.
    writePackage('design-system', './index.cjs', {
      'index.cjs': '',
      'index.mjs': 'export * from "./child.js"; import("./lazy.js")',
      'child.js':
        'export * from "./index.mjs"; import "@devup-ui/nested"; import "rejected"',
      'lazy.js': 'export const lazy = 1',
      'node_modules/@devup-ui/nested/package.json': '{"main":"index.js"}',
      'node_modules/@devup-ui/nested/index.js': 'export const nested = 1',
      'node_modules/rejected/package.json': '{"main":"index.js"}',
      'node_modules/rejected/index.js': 'export const rejected = 1',
    })
    // When collecting the complete extraction set.
    const files = collectProductionPrewarmFiles({
      cwd,
      graph: makeGraph([], ['design-system']),
      expectedBaseFiles: [],
      libPackage: '@devup-ui/react',
      include: ['design-system'],
    })
    // Then the closure is complete, deduplicated and policy-filtered.
    expect(files).toEqual([
      'node_modules/design-system/child.js',
      'node_modules/design-system/index.mjs',
      'node_modules/design-system/lazy.js',
      'node_modules/design-system/node_modules/@devup-ui/nested/index.js',
    ])
  })

  it('propagates a genuine accepted-package collection error', () => {
    // Given a malformed accepted package manifest.
    writePackage('design-system', './index.js', { 'index.js': '' })
    writeFileSync(join(cwd, 'node_modules/design-system/package.json'), '{')
    // When collecting it, then the build cannot silently omit its styles.
    expect(() =>
      collectProductionPrewarmFiles({
        cwd,
        graph: makeGraph([], ['design-system']),
        expectedBaseFiles: [],
        libPackage: '@devup-ui/react',
        include: ['design-system'],
      }),
    ).toThrow()
  })
})
