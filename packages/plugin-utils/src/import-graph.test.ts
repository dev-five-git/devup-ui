import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  buildCanonicalMap,
  computeFileReach,
  computeFileRoutes,
} from './import-graph'

describe('buildCanonicalMap', () => {
  let tempRoot: string
  let cwd: string
  let srcDir: string

  beforeEach(() => {
    tempRoot = mkdtempSync(join(tmpdir(), 'devup-ui-import-graph-'))
    cwd = join(tempRoot, 'project')
    srcDir = join(cwd, 'src')
    mkdirSync(srcDir, { recursive: true })
  })

  afterEach(() => {
    rmSync(tempRoot, { recursive: true, force: true })
  })

  function writeFixture(path: string, code: string): void {
    const filePath = join(cwd, path)
    mkdirSync(dirname(filePath), { recursive: true })
    writeFileSync(filePath, code)
  }

  it('should collapse a file with a single static importer', () => {
    writeFixture('src/a.tsx', "import './b'\n")
    writeFixture('src/b.tsx', 'export const b = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir })

    expect(map).toEqual({
      'src/b.tsx': 'src/a.tsx',
    })
  })

  it('should keep files with at least two static importers split', () => {
    writeFixture('src/a.tsx', "import './c'\n")
    writeFixture('src/d.tsx', "import './c'\n")
    writeFixture('src/c.tsx', 'export const c = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir })

    expect(map).toEqual({})
  })

  it('should keep dynamic import targets split', () => {
    writeFixture(
      'src/a.tsx',
      "export async function load() { return import('./e') }\n",
    )
    writeFixture('src/e.tsx', 'export const e = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir })

    expect(map).toEqual({})
  })

  it('should keep Next App Router special files as roots', () => {
    const routeFiles = [
      'page',
      'layout',
      'template',
      'default',
      'loading',
      'error',
      'not-found',
      'global-error',
    ]
    writeFixture(
      'src/app/importer.tsx',
      routeFiles.map((file) => `import './${file}'`).join('\n'),
    )
    for (const file of routeFiles) {
      writeFixture(`src/app/${file}.tsx`, `export const name = '${file}'\n`)
    }

    const map = buildCanonicalMap({ cwd, srcDir })

    expect(map).toEqual({})
  })

  it('should collapse single-importer chains to the top bucket root', () => {
    writeFixture('src/a.tsx', "import './b'\n")
    writeFixture('src/b.tsx', "import './c'\n")
    writeFixture('src/c.tsx', "import './d'\n")
    writeFixture('src/d.tsx', 'export const d = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir })

    expect(map).toEqual({
      'src/b.tsx': 'src/a.tsx',
      'src/c.tsx': 'src/a.tsx',
      'src/d.tsx': 'src/a.tsx',
    })
  })

  it('should keep closed cycles split without hanging', () => {
    writeFixture('src/a.tsx', "import './b'\n")
    writeFixture('src/b.tsx', "import './a'\n")

    const map = buildCanonicalMap({ cwd, srcDir })

    expect(map).toEqual({})
  })

  it('should return cwd-relative POSIX paths for keys and values', () => {
    writeFixture('src/app/a.tsx', "import '../shared/b'\n")
    writeFixture('src/shared/b.tsx', 'export const b = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir })

    const [key, value] = Object.entries(map)[0]

    expect(key).toBe('src/shared/b.tsx')
    expect(value).toBe('src/app/a.tsx')
  })

  it('should resolve tsconfig paths aliases inside srcDir', () => {
    writeFixture(
      'tsconfig.json',
      JSON.stringify({
        compilerOptions: {
          baseUrl: '.',
          paths: {
            '@/*': ['src/*'],
          },
        },
      }),
    )
    writeFixture('src/a.tsx', "import '@/foo'\n")
    writeFixture('src/foo.tsx', 'export const foo = 1\n')

    const map = buildCanonicalMap({
      cwd,
      srcDir,
      tsconfigPath: join(cwd, 'tsconfig.json'),
    })

    expect(map).toEqual({
      'src/foo.tsx': 'src/a.tsx',
    })
  })

  it('should ignore external package imports', () => {
    writeFixture('src/a.tsx', "import React from 'react'\n")

    const map = buildCanonicalMap({ cwd, srcDir })

    expect(map).toEqual({})
  })

  it('should ignore non-JavaScript import targets', () => {
    writeFixture('src/a.tsx', "import './x.css'\n")
    writeFixture('src/x.css', '.x { color: red }\n')

    const map = buildCanonicalMap({ cwd, srcDir })
    expect(map).toEqual({})
  })

  it('should not hoist shared route imports when hoistV is undefined', () => {
    writeFixture(
      'src/app/alpha/page.tsx',
      "import '../../components/shared'\nimport './private'\n",
    )
    writeFixture('src/app/alpha/private.tsx', 'export const privateValue = 1\n')
    writeFixture('src/app/beta/page.tsx', "import '../../components/shared'\n")
    writeFixture('src/components/shared.tsx', 'export const shared = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir })

    expect(map).toEqual({
      'src/app/alpha/private.tsx': 'src/app/alpha/page.tsx',
    })
  })

  it('should hoist a component reached by three routes when hoistV is large', () => {
    writeFixture('src/app/alpha/page.tsx', "import '../../components/shared'\n")
    writeFixture('src/app/beta/page.tsx', "import '../../components/shared'\n")
    writeFixture('src/app/gamma/page.tsx', "import '../../components/shared'\n")
    writeFixture('src/components/shared.tsx', 'export const shared = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir, hoistV: 100 })

    expect(map).toEqual({
      'src/components/shared.tsx': '@global',
    })
  })

  it('should not hoist a component below the hoistV one threshold', () => {
    writeFixture('src/app/alpha/page.tsx', "import '../../components/shared'\n")
    writeFixture('src/app/beta/page.tsx', "import '../../components/shared'\n")
    writeFixture('src/app/gamma/page.tsx', 'export const gamma = 1\n')
    writeFixture('src/components/shared.tsx', 'export const shared = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir, hoistV: 1 })

    expect(map).toEqual({})
  })

  it('should keep a component reached by one route private regardless of hoistV', () => {
    writeFixture('src/app/alpha/page.tsx', "import './private'\n")
    writeFixture('src/app/alpha/private.tsx', 'export const privateValue = 1\n')
    writeFixture('src/app/beta/page.tsx', 'export const beta = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir, hoistV: 100 })

    expect(map).toEqual({
      'src/app/alpha/private.tsx': 'src/app/alpha/page.tsx',
    })
  })

  it('should hoist files at the route reachability threshold boundary', () => {
    writeFixture(
      'src/app/alpha/page.tsx',
      "import '../../components/shared'\nimport './private'\n",
    )
    writeFixture('src/app/alpha/private.tsx', 'export const privateValue = 1\n')
    writeFixture('src/app/beta/page.tsx', "import '../../components/shared'\n")
    writeFixture('src/app/gamma/page.tsx', 'export const gamma = 1\n')
    writeFixture('src/app/delta/page.tsx', 'export const delta = 1\n')
    writeFixture('src/components/shared.tsx', 'export const shared = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir, hoistV: 2 })

    expect(map).toEqual({
      'src/app/alpha/private.tsx': 'src/app/alpha/page.tsx',
      'src/components/shared.tsx': '@global',
    })
  })

  it('should not count dynamic import targets as statically reached for hoist', () => {
    writeFixture(
      'src/app/alpha/page.tsx',
      "export async function load() { return import('./dynamic') }\n",
    )
    writeFixture('src/app/beta/page.tsx', 'export const beta = 1\n')
    writeFixture('src/app/alpha/dynamic.tsx', 'export const dynamic = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir, hoistV: 100 })

    expect(map).toEqual({})
  })

  it('should prefer @global over single-importer collapse for shared descendants', () => {
    writeFixture(
      'src/app/alpha/page.tsx',
      "import '../../components/shared-parent'\n",
    )
    writeFixture(
      'src/app/beta/page.tsx',
      "import '../../components/shared-parent'\n",
    )
    writeFixture('src/components/shared-parent.tsx', "import './shared-leaf'\n")
    writeFixture(
      'src/components/shared-leaf.tsx',
      'export const sharedLeaf = 1\n',
    )

    const map = buildCanonicalMap({ cwd, srcDir, hoistV: 100 })

    expect(map).toEqual({
      'src/components/shared-leaf.tsx': '@global',
      'src/components/shared-parent.tsx': '@global',
    })
  })

  it('should hoist components imported by ancestor layouts across leaf routes', () => {
    writeFixture('src/app/layout.tsx', "import './header'\n")
    writeFixture('src/app/header.tsx', 'export const Header = 1\n')
    writeFixture('src/app/a/page.tsx', 'export const A = 1\n')
    writeFixture('src/app/b/page.tsx', 'export const B = 1\n')
    writeFixture('src/app/c/page.tsx', 'export const C = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir, hoistV: 2 })

    expect(map).toEqual({
      'src/app/header.tsx': '@global',
      'src/app/layout.tsx': '@global',
    })
  })

  it('should keep one leaf route imports private under leaf route reachability', () => {
    writeFixture('src/app/a/page.tsx', "import './private'\n")
    writeFixture('src/app/a/private.tsx', 'export const privateValue = 1\n')
    writeFixture('src/app/b/page.tsx', 'export const B = 1\n')
    writeFixture('src/app/c/page.tsx', 'export const C = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir, hoistV: 100 })

    expect(map).toEqual({
      'src/app/a/private.tsx': 'src/app/a/page.tsx',
    })
  })

  it('should hoist nested layout imports for every leaf route they wrap', () => {
    writeFixture('src/app/layout.tsx', "import './header'\n")
    writeFixture('src/app/header.tsx', 'export const Header = 1\n')
    writeFixture('src/app/docs/layout.tsx', "import './sidebar'\n")
    writeFixture('src/app/docs/sidebar.tsx', 'export const Sidebar = 1\n')
    writeFixture('src/app/docs/x/page.tsx', "import './x-only'\n")
    writeFixture('src/app/docs/x/x-only.tsx', 'export const XOnly = 1\n')
    writeFixture('src/app/docs/y/page.tsx', 'export const DocsY = 1\n')
    writeFixture('src/app/other/page.tsx', 'export const Other = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir, hoistV: 100 })

    expect(map).toEqual({
      'src/app/docs/layout.tsx': '@global',
      'src/app/docs/sidebar.tsx': '@global',
      'src/app/docs/x/x-only.tsx': 'src/app/docs/x/page.tsx',
      'src/app/header.tsx': '@global',
      'src/app/layout.tsx': '@global',
    })
  })
})

describe('computeFileRoutes', () => {
  let tempRoot: string
  let cwd: string
  let srcDir: string

  beforeEach(() => {
    tempRoot = mkdtempSync(join(tmpdir(), 'devup-ui-file-routes-'))
    cwd = join(tempRoot, 'project')
    srcDir = join(cwd, 'src')
    mkdirSync(srcDir, { recursive: true })
  })

  afterEach(() => {
    rmSync(tempRoot, { recursive: true, force: true })
  })

  function writeFixture(path: string, code: string): void {
    const filePath = join(cwd, path)
    mkdirSync(dirname(filePath), { recursive: true })
    writeFileSync(filePath, code)
  }

  it('maps each route-private file to only its own leaf route id', () => {
    // route ids assigned by sorted leaf-route order: a/page=0, b/page=1
    writeFixture('src/app/a/page.tsx', "import './a-only'\n")
    writeFixture('src/app/a/a-only.tsx', 'export const A = 1\n')
    writeFixture('src/app/b/page.tsx', "import './b-only'\n")
    writeFixture('src/app/b/b-only.tsx', 'export const B = 1\n')

    const routes = computeFileRoutes({ cwd, srcDir })

    expect(routes['src/app/a/page.tsx']).toEqual([0])
    expect(routes['src/app/a/a-only.tsx']).toEqual([0])
    expect(routes['src/app/b/page.tsx']).toEqual([1])
    expect(routes['src/app/b/b-only.tsx']).toEqual([1])
  })

  it('assigns a shared layout/component to every leaf route it wraps', () => {
    writeFixture('src/app/layout.tsx', "import './shared'\n")
    writeFixture('src/app/shared.tsx', 'export const Shared = 1\n')
    writeFixture('src/app/a/page.tsx', 'export const A = 1\n')
    writeFixture('src/app/b/page.tsx', 'export const B = 1\n')

    const routes = computeFileRoutes({ cwd, srcDir })

    // layout + its import wrap BOTH leaf routes (0 and 1) -> hoist candidates
    expect(routes['src/app/layout.tsx']).toEqual([0, 1])
    expect(routes['src/app/shared.tsx']).toEqual([0, 1])
    // leaf-private files stay single-route
    expect(routes['src/app/a/page.tsx']).toEqual([0])
    expect(routes['src/app/b/page.tsx']).toEqual([1])
  })

  it('unions route ids for a component imported by multiple routes', () => {
    writeFixture('src/app/a/page.tsx', "import '../../shared/card'\n")
    writeFixture('src/app/b/page.tsx', "import '../../shared/card'\n")
    writeFixture('src/app/c/page.tsx', 'export const C = 1\n')
    writeFixture('src/shared/card.tsx', 'export const Card = 1\n')

    const routes = computeFileRoutes({ cwd, srcDir })

    // card is used by a/page (0) and b/page (1), not c/page (2)
    expect(routes['src/shared/card.tsx']).toEqual([0, 1])
    expect(routes['src/app/c/page.tsx']).toEqual([2])
  })

  it('omits files reachable from no leaf route', () => {
    writeFixture('src/app/a/page.tsx', 'export const A = 1\n')
    writeFixture('src/orphan.tsx', 'export const Orphan = 1\n')

    const routes = computeFileRoutes({ cwd, srcDir })

    expect(routes['src/orphan.tsx']).toBeUndefined()
    expect(routes['src/app/a/page.tsx']).toEqual([0])
  })
})

describe('computeFileReach', () => {
  let tempRoot: string
  let cwd: string
  let srcDir: string

  beforeEach(() => {
    tempRoot = mkdtempSync(join(tmpdir(), 'devup-ui-file-reach-'))
    cwd = join(tempRoot, 'project')
    srcDir = join(cwd, 'src')
    mkdirSync(srcDir, { recursive: true })
  })

  afterEach(() => {
    rmSync(tempRoot, { recursive: true, force: true })
  })

  function writeFixture(path: string, code: string): void {
    const filePath = join(cwd, path)
    mkdirSync(dirname(filePath), { recursive: true })
    writeFileSync(filePath, code)
  }

  it('treats 0-importer files as entries and shares a common dep across them', () => {
    // entries (no importer): main-a, main-b (sorted -> a=0, b=1). shared imported by both.
    writeFixture('src/main-a.tsx', "import './shared'\n")
    writeFixture('src/main-b.tsx', "import './shared'\n")
    writeFixture('src/shared.tsx', 'export const S = 1\n')

    const reach = computeFileReach({ cwd, srcDir })

    expect(reach['src/main-a.tsx']).toEqual([0])
    expect(reach['src/main-b.tsx']).toEqual([1])
    // shared dep reached by BOTH entries -> hoist candidate
    expect(reach['src/shared.tsx']).toEqual([0, 1])
  })

  it('gives single-entry SPA reach 1 for everything (nothing hoists)', () => {
    writeFixture('src/index.tsx', "import './a'\n")
    writeFixture('src/a.tsx', "import './b'\n")
    writeFixture('src/b.tsx', 'export const B = 1\n')

    const reach = computeFileReach({ cwd, srcDir })

    expect(reach['src/index.tsx']).toEqual([0])
    expect(reach['src/a.tsx']).toEqual([0])
    expect(reach['src/b.tsx']).toEqual([0])
  })

  it('treats dynamic-import targets as their own entries', () => {
    writeFixture(
      'src/index.tsx',
      "export const load = () => import('./lazy')\n",
    )
    writeFixture('src/lazy.tsx', 'export const L = 1\n')

    const reach = computeFileReach({ cwd, srcDir })

    // index (0-importer) and lazy (dynamic target) are both entries
    expect(Object.keys(reach)).toContain('src/index.tsx')
    expect(Object.keys(reach)).toContain('src/lazy.tsx')
  })

  it('honors an explicit entries override', () => {
    writeFixture('src/index.tsx', "import './a'\nimport './b'\n")
    writeFixture('src/a.tsx', 'export const A = 1\n')
    writeFixture('src/b.tsx', 'export const B = 1\n')

    // override: pretend a and b are the real entries (e.g. MPA inputs)
    const reach = computeFileReach({
      cwd,
      srcDir,
      entries: ['src/a.tsx', 'src/b.tsx'],
    })

    expect(reach['src/a.tsx']).toEqual([0])
    expect(reach['src/b.tsx']).toEqual([1])
    // index is not an entry now and nobody it imports... it's not in any entry closure
    expect(reach['src/index.tsx']).toBeUndefined()
  })
})
