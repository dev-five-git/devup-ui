import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  computeCompiledFiles,
  computeFileReach,
  computeFileRoutes,
  computeReachableFiles,
  createModuleResolver,
} from '../import-graph'
import { SOURCE_EXTENSIONS } from '../shared'

const parent = join(
  tmpdir(),
  'opencode',
  'workers',
  'w20-plugins-core',
  'shared',
)
let root: string
beforeEach(() => {
  mkdirSync(parent, { recursive: true })
  root = realpathSync(mkdtempSync(join(parent, 'graph-')))
})
afterEach(() => rmSync(root, { recursive: true, force: true }))

function fixture(path: string, source: string): string {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, source)
  return file
}

describe('multi-root source graph', () => {
  it('resolves dotted specifiers and directories without replacing real asset extensions', () => {
    const entry = fixture(
      'src/main.ts',
      "import './a.generated'\nimport './b.config'\nimport './folder.with.dot'\nimport './c.css'",
    )
    const generated = fixture(
      'src/a.generated.ts',
      'export const generated = 1',
    )
    const config = fixture('src/b.config.tsx', 'export const config = 1')
    const index = fixture(
      'src/folder.with.dot/index.ts',
      'export const index = 1',
    )
    const css = fixture('src/c.css', 'body{color:red}')
    fixture('src/c.css.ts', 'export const competing = 1')
    const resolver = createModuleResolver({ cwd: root })
    expect(resolver('./a.generated', entry)?.path).toBe(generated)
    expect(resolver('./b.config', entry)?.path).toBe(config)
    expect(resolver('./folder.with.dot', entry)?.path).toBe(index)
    expect(resolver('./c.css', entry)?.path).toBe(css)
    const graph = buildStaticImportGraph(join(root, 'src'), undefined, {
      cwd: root,
    })
    expect(graph.staticImports.get(entry)).toEqual(
      new Set([generated, config, index]),
    )
    expect(graph.files).not.toContain(css)
    expect(
      computeReachableFiles({
        srcDir: join(root, 'src'),
        graph,
        entries: [join(root, 'src/a.generated')],
      }),
    ).toEqual([generated])
  })

  it('follows only eligible package closures when an entry uses the project root layout', () => {
    const entry = fixture('entry.ts', "import 'included'\nimport 'excluded'")
    fixture(
      'node_modules/included/package.json',
      JSON.stringify({ main: 'index.js' }),
    )
    const included = fixture(
      'node_modules/included/index.js',
      "import './leaf.cjs'",
    )
    const leaf = fixture(
      'node_modules/included/leaf.cjs',
      'export const value = 1',
    )
    fixture(
      'node_modules/excluded/package.json',
      JSON.stringify({ main: 'index.js' }),
    )
    fixture('node_modules/excluded/index.js', 'export const value = 2')
    const graph = buildStaticImportGraph(root, undefined, {
      cwd: root,
      include: ['included'],
    })
    expect(graph.files).toEqual([entry, included, leaf].sort())
    expect(graph.staticImports.get(entry)).toEqual(new Set([included]))
    expect(graph.externalImports?.get(entry)).toEqual(new Set(['excluded']))
  })

  it('shares one graph across reach, route, compiled and collapse consumers', () => {
    const entry = fixture(
      'app/page.mts',
      "import '../src/value.cts'\nimport('included')",
    )
    const value = fixture('src/value.cts', 'export const value = 1')
    fixture(
      'node_modules/included/package.json',
      JSON.stringify({
        exports: { browser: './browser.mjs', import: './server.js' },
      }),
    )
    const library = fixture(
      'node_modules/included/browser.mjs',
      "export { value } from './leaf.cjs'\nimport('other')",
    )
    const leaf = fixture(
      'node_modules/included/leaf.cjs',
      'export const value = 2',
    )
    fixture('node_modules/included/unreachable.ts', 'unused')
    fixture('node_modules/included/server.js', 'server')
    fixture(
      'node_modules/other/package.json',
      JSON.stringify({ main: 'index.js' }),
    )
    const other = fixture('node_modules/other/index.js', 'other')
    const srcDir = ['src', 'app']
    const graph = buildStaticImportGraph(srcDir, undefined, {
      cwd: root,
      include: ['included'],
      conditions: ['browser', 'import'],
    })
    expect(graph.files).toEqual([entry, library, leaf, value].sort())
    expect(graph.externalImports?.get(library)).toEqual(new Set(['other']))
    expect(graph.fileSet.has(other)).toBe(false)
    expect(computeReachableFiles({ srcDir, graph, entries: [entry] })).toEqual(
      graph.files,
    )
    const opts = { cwd: root, srcDir, graph }
    expect(buildCanonicalMap(opts)['src/value.cts']).toBe('app/page.mts')
    expect(
      computeFileReach({ ...opts, entries: ['app/page.mts'] })['src/value.cts'],
    ).toEqual([0])
    expect(computeFileRoutes(opts)['src/value.cts']).toEqual([0])
    expect(computeCompiledFiles(opts)).toEqual(
      graph.files
        .map((file) => file.slice(root.length + 1).replaceAll('\\', '/'))
        .sort(),
    )
  })
  it('follows reachable static included dependencies without enumerating their package', () => {
    const entry = fixture('src/main.ts', "import '@scope/pkg'")
    fixture(
      'node_modules/@scope/pkg/package.json',
      JSON.stringify({ main: 'index.ts' }),
    )
    const library = fixture(
      'node_modules/@scope/pkg/index.ts',
      "import './nested/a'\nimport('included-two')",
    )
    const nested = fixture(
      'node_modules/@scope/pkg/nested/a.ts',
      'export const a = 1',
    )
    fixture(
      'node_modules/included-two/package.json',
      JSON.stringify({ main: 'index.ts' }),
    )
    const second = fixture(
      'node_modules/included-two/index.ts',
      'export const second = 1',
    )
    fixture('node_modules/@scope/pkg/orphan.ts', 'orphan')
    const graph = buildStaticImportGraph(join(root, 'src'), undefined, {
      cwd: root,
      include: ['@scope/pkg', 'included-two'],
    })
    expect(
      computeReachableFiles({
        srcDir: join(root, 'src'),
        graph,
        entries: [entry],
      }),
    ).toEqual([library, nested, second, entry].sort())
    expect(
      buildCanonicalMap({ cwd: root, srcDir: join(root, 'src'), graph })[
        'node_modules/@scope/pkg/nested/a.ts'
      ],
    ).toBe('src/main.ts')
  })
  it.each(SOURCE_EXTENSIONS)(
    'discovers and resolves %s modules and index files',
    (extension) => {
      const entry = fixture(
        `src/main${extension}`,
        "import './value'\nimport './folder'",
      )
      const value = fixture(`src/value${extension}`, 'export const value = 1')
      const index = fixture(
        `src/folder/index${extension}`,
        'export const index = 1',
      )
      const graph = buildStaticImportGraph(join(root, 'src'))
      expect(graph.staticImports.get(entry)).toEqual(new Set([value, index]))
      const resolver = createModuleResolver({ cwd: root })
      expect(resolver(`./value${extension}`, entry)?.path).toBe(value)
    },
  )
})

describe('MDX ESM discovery', () => {
  it('discovers real ESM blocks while ignoring markdown, code fences, indented code and comments', () => {
    const entry = fixture(
      'app/page.mdx',
      [
        '# Heading',
        '',
        "Prose about import './fake'",
        '',
        '```tsx',
        "import './fake'",
        '```',
        '',
        '~~~js',
        "import './fake'",
        '~~~',
        '',
        "    import './fake'",
        '',
        '<!--',
        "import './fake'",
        '-->',
        '',
        '<!-- single line -->',
        '',
        'import {',
        '  value',
        "} from '../src/value.mts'",
        '',
        "export { other } from '../src/other.cts'",
        '',
        "export const load = () => import('../src/lazy.cjs')",
        '',
        '<div>Content</div>',
      ].join('\n'),
    )
    const value = fixture('src/value.mts', 'export const value = 1')
    const other = fixture('src/other.cts', 'export const other = 1')
    const lazy = fixture('src/lazy.cjs', 'export const lazy = 1')
    fixture('app/fake.ts', 'fake')
    const srcDir = [join(root, 'app'), join(root, 'src')]
    const graph = buildStaticImportGraph(srcDir, undefined, { cwd: root })
    expect(graph.staticImports.get(entry)).toEqual(new Set([value, other]))
    expect(graph.dynamicImports.get(entry)).toEqual(new Set([lazy]))
    expect(computeCompiledFiles({ srcDir, cwd: root, graph })).toEqual([
      'app/page.mdx',
      'src/lazy.cjs',
      'src/other.cts',
      'src/value.mts',
    ])
  })
})

describe('conditional package resolution', () => {
  it('propagates invalid module path I/O errors instead of reporting an unresolved import', () => {
    fixture('src/main.ts', 'export const main = 1')
    expect(() =>
      createModuleResolver({ cwd: root })('./\0.ts', 'src/main.ts'),
    ).toThrow()
  })
  it('honors active conditions in declaration order rather than condition input order', () => {
    fixture(
      'node_modules/pkg/package.json',
      JSON.stringify({
        exports: {
          browser: './browser.js',
          import: './import.js',
          default: './default.js',
        },
      }),
    )
    fixture('node_modules/pkg/browser.js', 'browser')
    fixture('node_modules/pkg/import.js', 'import')
    fixture('node_modules/pkg/default.js', 'default')
    expect(
      createModuleResolver({ cwd: root, conditions: ['import', 'browser'] })(
        'pkg',
        'src/main.ts',
      )?.code,
    ).toBe('browser')
    expect(
      createModuleResolver({ cwd: root, conditions: [] })('pkg', 'src/main.ts')
        ?.code,
    ).toBe('default')
  })
  it('honors early default branches and null exclusions', () => {
    fixture(
      'node_modules/pkg/package.json',
      JSON.stringify({
        exports: {
          '.': { default: './default.js', browser: './browser.js' },
          './blocked': { browser: null, default: './default.js' },
          './empty': [],
        },
      }),
    )
    fixture('node_modules/pkg/default.js', 'default')
    const resolver = createModuleResolver({
      cwd: root,
      conditions: ['browser'],
    })
    expect(resolver('pkg', 'src/main.ts')?.code).toBe('default')
    expect(resolver('pkg/blocked', 'src/main.ts')).toBeUndefined()
    expect(resolver('pkg/empty', 'src/main.ts')).toBeUndefined()
  })
  it.each(['{', 'null', '[]'])(
    'propagates invalid manifest %s with location and cause',
    (content) => {
      const file = fixture('node_modules/pkg/package.json', content)
      expect(() =>
        createModuleResolver({ cwd: root })('pkg', 'src/main.ts'),
      ).toThrow(file)
    },
  )
})
