import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

import type { StaticImportGraph } from '@devup-ui/plugin-utils'
import { beforeEach, describe, expect, it } from 'bun:test'

import {
  collectPrewarmFiles,
  type CollectPrewarmFilesOptions,
  EXTRACTABLE_EXTENSION,
} from '../prewarm'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()

let root: string

beforeEach(() => {
  root = makeProject()
})

function write(path: string, contents = ''): void {
  mkdirSync(dirname(join(root, path)), { recursive: true })
  writeFileSync(join(root, path), contents)
}

function writePackage(
  name: string,
  exports: Record<string, unknown> | string,
  files: Record<string, string>,
): void {
  write(`node_modules/${name}/package.json`, JSON.stringify({ name, exports }))
  for (const [filename, contents] of Object.entries(files)) {
    write(`node_modules/${name}/${filename}`, contents)
  }
}

function makeGraph(
  imports: Record<string, string[]>,
  extra: string[] = [],
): StaticImportGraph {
  const files = [...Object.keys(imports), ...extra].map((file) =>
    join(root, file),
  )
  return {
    files,
    fileSet: new Set(files),
    staticImports: new Map(files.map((file) => [file, new Set()])),
    staticImporters: new Map(files.map((file) => [file, new Set()])),
    dynamicTargets: new Set(),
    dynamicImports: new Map(files.map((file) => [file, new Set()])),
    externalImports: new Map(
      Object.entries(imports).map(([file, specifiers]) => [
        join(root, file),
        new Set(specifiers),
      ]),
    ),
  }
}

function collect(
  graph: StaticImportGraph,
  overrides: Partial<CollectPrewarmFilesOptions> = {},
): string[] {
  return collectPrewarmFiles({
    root,
    graph,
    expectedBaseFiles: ['src/app/page.tsx'],
    libPackage: '@devup-ui/react',
    include: [],
    prewarmAll: false,
    ...overrides,
  })
}

describe('collectPrewarmFiles', () => {
  it('extracts the proven compiled closure and not the files nothing compiled', () => {
    const graph = makeGraph({}, ['src/app/page.tsx', 'src/dead/broken.tsx'])

    expect(collect(graph)).toEqual(['src/app/page.tsx'])
  })

  it('follows only the packages the reached files import', () => {
    writePackage('@acme/ui', './index.js', { 'index.js': '' })
    writePackage('@acme/dead', './index.js', { 'index.js': '' })
    const graph = makeGraph({
      'src/app/page.tsx': ['@acme/ui'],
      'src/dead/broken.tsx': ['@acme/dead'],
    })

    expect(collect(graph, { include: ['@acme/ui', '@acme/dead'] })).toEqual([
      'node_modules/@acme/ui/index.js',
      'src/app/page.tsx',
    ])
  })

  it('opts into the whole tree with prewarmAll', () => {
    writePackage('@acme/dead', './index.js', { 'index.js': '' })
    const graph = makeGraph({
      'src/app/page.tsx': [],
      'src/dead/broken.tsx': ['@acme/dead'],
    })

    expect(
      collect(graph, { prewarmAll: true, include: ['@acme/dead'] }),
    ).toEqual([
      'node_modules/@acme/dead/index.js',
      'src/app/page.tsx',
      'src/dead/broken.tsx',
    ])
  })

  it('accepts the packages the loader would extract, with their ESM entries', () => {
    writePackage(
      '@devup-ui/reset-css',
      { '.': { import: './dist/index.mjs', require: './dist/index.cjs' } },
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
    writePackage('@devup-ui/types-only', './index.d.ts', { 'index.d.ts': '' })
    writePackage('@devup-ui/data', './data.json', { 'data.json': '{}' })
    writePackage('react', './index.js', { 'index.js': '' })

    const files = collect(
      makeGraph({
        'src/app/page.tsx': [
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
          '@devup-ui/types-only',
          '@devup-ui/data',
          '@devup-ui/missing',
        ],
      }),
      { libPackage: '@acme/ui', include: ['design-system'] },
    )

    expect(files).toEqual([
      'node_modules/@acme/ui/index.mjs',
      'node_modules/@devup-editor/editor/index.js',
      'node_modules/@devup-ui/cjs-only/index.cjs',
      'node_modules/@devup-ui/reset-css/dist/index.mjs',
      'node_modules/design-system/index.js',
      'src/app/page.tsx',
    ])
  })

  it('prefers the ESM sibling of a CommonJS entry', () => {
    writePackage('@devup-ui/dual', './index.cjs', {
      'index.cjs': '',
      'index.mjs': '',
    })

    expect(
      collect(makeGraph({ 'src/app/page.tsx': ['@devup-ui/dual'] })),
    ).toEqual(['node_modules/@devup-ui/dual/index.mjs', 'src/app/page.tsx'])
  })

  it('follows a package through its own imports, once each', () => {
    writePackage('@acme/ui', './index.mjs', {
      'index.mjs': [
        "import './button.mjs'",
        "export * from './nested'",
        "import { x } from '@acme/shared'",
        "import { y } from '@devup-ui/reset-css'",
        "import react from 'react'",
        "import data from './data.json'",
        "const lazy = () => import('./lazy.mts')",
        "const cjs = require('./legacy.cjs')",
        "import './missing'",
      ].join('\n'),
      'button.mjs': "import './index.mjs'\nimport './nested'",
      'nested/index.js': "export * from '../button.mjs'",
      'lazy.mts': '',
      'legacy.cjs': '',
      'data.json': '{}',
    })
    writePackage('@acme/shared', './index.js', { 'index.js': '' })
    writePackage('@devup-ui/reset-css', './index.js', { 'index.js': '' })
    writePackage('react', './index.js', { 'index.js': '' })

    expect(
      collect(makeGraph({ 'src/app/page.tsx': ['@acme/ui'] }), {
        include: ['@acme/ui', '@acme/shared'],
      }),
    ).toEqual([
      'node_modules/@acme/shared/index.js',
      'node_modules/@acme/ui/button.mjs',
      'node_modules/@acme/ui/index.mjs',
      'node_modules/@acme/ui/legacy.cjs',
      'node_modules/@acme/ui/nested/index.js',
      'node_modules/@devup-ui/reset-css/index.js',
      'src/app/page.tsx',
    ])
  })

  it('names the importer when a package cannot be resolved', () => {
    write('node_modules/@acme/ui/package.json', '{ not json')
    const importer = join(root, 'src/app/page.tsx')

    expect(() =>
      collect(makeGraph({ 'src/app/page.tsx': ['@acme/ui'] }), {
        include: ['@acme/ui'],
      }),
    ).toThrow(
      new RegExp(
        `${importer.replaceAll('\\', '\\\\')}:1:1: devup-ui prewarm cannot use \`@acme/ui\` at build time: .*; needs a resolvable package`,
      ),
    )
  })

  it('normalizes an absolute expected file and tolerates a graph without externals', () => {
    const graph = makeGraph({})
    delete graph.externalImports

    expect(
      collect(graph, {
        expectedBaseFiles: [join(root, 'src/app/page.tsx')],
        libPackage: '@',
      }),
    ).toEqual(['src/app/page.tsx'])
  })

  it('is the same whatever order the closure is given in', () => {
    const graph = makeGraph({}, ['src/b.tsx', 'src/a.tsx'])

    expect(
      collect(graph, { expectedBaseFiles: ['src/b.tsx', 'src/a.tsx'] }),
    ).toEqual(['src/a.tsx', 'src/b.tsx'])
  })
})

describe('EXTRACTABLE_EXTENSION', () => {
  it.each([
    'a.ts',
    'a.tsx',
    'a.js',
    'a.jsx',
    'a.mjs',
    'a.cjs',
    'a.mts',
    'a.cts',
  ])('accepts %s', (name) =>
    expect(EXTRACTABLE_EXTENSION.test(name)).toBe(true),
  )
  it.each(['a.json', 'a.css', 'a.mdx', 'a.node', 'a'])('rejects %s', (name) =>
    expect(EXTRACTABLE_EXTENSION.test(name)).toBe(false),
  )
})
