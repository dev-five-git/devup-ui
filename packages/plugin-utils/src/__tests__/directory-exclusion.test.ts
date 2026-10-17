import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, expect, it, spyOn } from 'bun:test'

import { createDirectoryExclusion } from '../directory-exclusion'
import { buildStaticImportGraph, listSourceFiles } from '../import-graph'
import { collectNumberedFiles } from '../numbering'

let root: string
beforeEach(() => {
  const parent = join(
    tmpdir(),
    'opencode',
    'workers',
    'w20-plugins-core',
    'shared-api',
  )
  fs.mkdirSync(parent, { recursive: true })
  root = fs.realpathSync.native(fs.mkdtempSync(join(parent, 'exclude-')))
})
afterEach(() => fs.rmSync(root, { recursive: true, force: true }))
function file(path: string, source = 'export {}'): string {
  const target = join(root, path)
  fs.mkdirSync(dirname(target), { recursive: true })
  fs.writeFileSync(target, source)
  return target
}

it('matches bare names at every depth and absolute directories only within their boundary', () => {
  const match = createDirectoryExclusion(
    ['generated', '/project/build/next'],
    'linux',
  )
  expect(match('/project/src/nested/generated')).toBe(true)
  expect(match('/project/build/next/deep')).toBe(true)
  expect(match('/project/other/next')).toBe(false)
  expect(match('/project/build/next-other')).toBe(false)
  expect(match('/project/Generated')).toBe(false)
})

it('normalizes Windows separators, drive case and directory case but retains boundaries', () => {
  const match = createDirectoryExclusion(
    ['Generated', 'C:/Project/build/Next/'],
    'win32',
  )
  expect(match('c:\\PROJECT\\BUILD\\next\\deep')).toBe(true)
  expect(match('D:/project/deep/GENERATED')).toBe(true)
  expect(match('C:/Project/other/Next')).toBe(false)
  expect(match('c:/project/build/next-other')).toBe(false)
  expect(createDirectoryExclusion(['C:/'], 'win32')('c:/project')).toBe(true)
})

it('keeps a same-basename source folder while never reading, parsing, preparing or numbering excluded source', async () => {
  const entry = file(
    'src/main.ts',
    "import '../build/next/poison'\nimport './next/value'\nimport './nested/generated/poison'\nimport 'provider'\nimport 'included'",
  )
  const kept = file('src/next/value.ts')
  const poison = file('build/next/poison.ts', "import 'invalid-manifest'")
  file('src/nested/generated/poison.ts', "import 'invalid-manifest'")
  file('node_modules/invalid-manifest/package.json', '{')
  file('node_modules/included/package.json', '{"main":"index.js"}')
  file('node_modules/included/index.js', "import 'invalid-manifest'")
  const excluded = [
    dirname(poison),
    'generated',
    join(root, 'node_modules/included'),
  ]
  const originalRead = fs.readFileSync
  const read = spyOn(fs, 'readFileSync').mockImplementation(
    new Proxy(originalRead, {
      apply(target, thisArg, args) {
        if (
          typeof args[0] === 'string' &&
          /poison|included[/\\]index/.test(args[0])
        )
          throw new Error('Excluded source read')
        return Reflect.apply(target, thisArg, args)
      },
    }),
  )
  try {
    const options = {
      cwd: root,
      exclude: excluded,
      alias: { provider: poison },
      include: ['included'],
    }
    const raw = buildStaticImportGraph('src', undefined, options)
    const visits: string[] = []
    const prepared = await buildStaticImportGraph('src', undefined, {
      ...options,
      prepareSource: (filename) => {
        visits.push(filename)
        return undefined
      },
    })
    expect(raw.files).toEqual([entry, kept].sort())
    expect(prepared.files).toEqual(raw.files)
    expect(visits.sort()).toEqual(raw.files)
    expect(
      collectNumberedFiles({
        roots: [join(root, 'src'), dirname(poison)],
        include: ['included'],
        cwd: root,
        exclude: excluded,
        needles: ['export'],
        toId: (path) => path,
      }),
    ).toEqual([kept])
    expect(listSourceFiles(dirname(poison), excluded)).toEqual([])
  } finally {
    read.mockRestore()
  }
})

it('uses native Windows absolute-path normalization in a real scanner fixture', () => {
  const excluded = file('build/next/poison.ts')
  const kept = file('src/next/value.ts')
  const path = dirname(excluded)
  const exclude =
    process.platform === 'win32'
      ? path.toUpperCase().replaceAll('\\', '/')
      : path
  expect(listSourceFiles(root, [exclude])).toEqual([kept])
})

it.each([
  'package',
  'package-alias',
  'package-subpath',
  'package-subpath-alias',
  'absolute',
  'absolute-alias',
])(
  'never reads a poisoned excluded manifest or source when resolving %s',
  async (kind) => {
    // Given an included package whose excluded manifest cannot be parsed.
    const manifest = file('node_modules/excluded/package.json', '{')
    const subpath = kind.includes('subpath')
    const poison = file(
      `node_modules/excluded/${subpath ? 'blocked/' : ''}index.ts`,
      "import 'invalid'",
    )
    const request = kind.startsWith('absolute')
      ? poison.replaceAll('\\', '/')
      : `excluded${subpath ? '/blocked/index' : ''}`
    const aliased = kind.endsWith('alias')
    const entry = file(
      'src/main.ts',
      `import '${aliased ? 'provider' : request}'`,
    )
    const excluded = subpath ? dirname(poison) : dirname(manifest)
    const alias: Readonly<Record<string, string>> = aliased
      ? { provider: request }
      : {}
    const options = {
      cwd: root,
      include: ['excluded'],
      exclude: [excluded],
      alias,
    }
    const raw = buildStaticImportGraph(['src', excluded], undefined, options)
    const match = createDirectoryExclusion([excluded])
    const originalRead = fs.readFileSync
    const read = spyOn(fs, 'readFileSync').mockImplementation(
      new Proxy(originalRead, {
        apply(target, thisArg, args) {
          if (typeof args[0] === 'string' && match(dirname(args[0])))
            throw new Error('Excluded manifest/source read')
          return Reflect.apply(target, thisArg, args)
        },
      }),
    )
    try {
      // When raw/prepared graphs and numbering follow the same excluded edge/root.
      const visits: string[] = []
      const prepared = await buildStaticImportGraph(
        ['src', excluded],
        undefined,
        {
          ...options,
          prepareSource: (filename) => {
            visits.push(filename)
            return undefined
          },
        },
      )
      const numbered = collectNumberedFiles({
        ...options,
        roots: [join(root, 'src'), excluded],
        needles: ['import'],
        toId: (path) => path,
      })
      // Then no excluded source is visited, prewarmed or needle-read.
      expect(raw.files).toEqual([entry])
      expect(prepared.files).toEqual([entry])
      expect(visits).toEqual([entry])
      expect(raw.externalImports?.get(entry)).toEqual(new Set())
      expect(prepared.externalImports?.get(entry)).toEqual(new Set())
      expect(numbered).toEqual([entry])
    } finally {
      read.mockRestore()
    }
  },
)

it.each(['paths', 'baseUrl'])(
  'keeps excluded %s candidates terminal rather than prewarming a package fallback',
  (kind) => {
    const poison = file('blocked/index.ts')
    const entry = file('src/main.ts', "import 'blocked'")
    file('node_modules/blocked/package.json', '{')
    file(
      'tsconfig.json',
      JSON.stringify({
        compilerOptions:
          kind === 'paths'
            ? { paths: { blocked: ['./blocked/index.ts'] } }
            : { baseUrl: '.' },
      }),
    )
    const graph = buildStaticImportGraph('src', undefined, {
      cwd: root,
      include: ['blocked'],
      exclude: [dirname(poison)],
    })
    expect(graph.files).toEqual([entry])
    expect(graph.externalImports?.get(entry)).toEqual(new Set())
  },
)
