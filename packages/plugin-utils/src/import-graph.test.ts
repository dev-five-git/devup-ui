import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import {
  __setOxcParserForTest,
  buildCanonicalMap,
  buildStaticImportGraph,
  computeFileReach,
  computeFileRoutes,
  planAtomHoist,
  runImportGraphCli,
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

  // Type-only imports/exports are erased by the bundler and produce NO runtime
  // module. Counting them as static graph edges merged phantom members into a
  // bucket the bundler never compiles, which forced the next-plugin coordinator
  // to wait out its wall-clock fail-open. They must NOT become graph edges.
  it('should not treat a named `import type` target as a bucket member', () => {
    writeFixture(
      'src/a.tsx',
      "import type { M } from './m'\nexport const a = 1\n",
    )
    writeFixture('src/m.tsx', 'export type M = number\nexport const m = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({})
  })

  it('should not treat a default `import type Foo` target as a member', () => {
    writeFixture(
      'src/a.tsx',
      "import type Foo from './foo'\nexport const a = 1\n",
    )
    writeFixture('src/foo.tsx', 'const foo = 1\nexport default foo\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({})
  })

  it('should not treat a namespace `import type * as NS` target as a member', () => {
    writeFixture(
      'src/a.tsx',
      "import type * as NS from './ns'\nexport const a = 1\n",
    )
    writeFixture('src/ns.tsx', 'export const ns = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({})
  })

  it('should not treat an `export type ... from` target as a member', () => {
    writeFixture(
      'src/b.tsx',
      "export type { M } from './m'\nexport const b = 1\n",
    )
    writeFixture('src/m.tsx', 'export type M = number\nexport const m = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({})
  })

  it('should keep an inline `import { type T, val }` target (module still imported)', () => {
    writeFixture(
      'src/a.tsx',
      "import { type T, val } from './b'\nexport const a: T = val\n",
    )
    writeFixture('src/b.tsx', 'export type T = number\nexport const val = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/b.tsx': 'src/a.tsx',
    })
  })

  it('should not treat an all-inline-type `import { type T }` target as a member', () => {
    // With no value bindings left, TypeScript import elision erases the whole
    // statement at build time — no runtime module, so no graph edge. Keeping
    // it made the target a phantom bucket member the bundler never compiles.
    writeFixture(
      'src/a.tsx',
      "import { type T } from './b'\nexport const a: T = 1\n",
    )
    writeFixture('src/b.tsx', 'export type T = number\nexport const val = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({})
  })

  it('should not treat a multiline all-inline-type import target as a member', () => {
    writeFixture(
      'src/a.tsx',
      "import {\n  type T,\n  type U,\n} from './b'\nexport const a = 1\n",
    )
    writeFixture(
      'src/b.tsx',
      'export type T = number\nexport type U = string\nexport const val = 1\n',
    )

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({})
  })

  it('should not treat an all-inline-type `export { type T } from` target as a member', () => {
    writeFixture(
      'src/a.tsx',
      "export { type T } from './b'\nexport const a = 1\n",
    )
    writeFixture('src/b.tsx', 'export type T = number\nexport const val = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({})
  })

  it('keeps a mixed `export { type T, x } from` target (module still imported)', () => {
    writeFixture('src/a.tsx', "export { type T, x } from './b'\n")
    writeFixture('src/b.tsx', 'export type T = number\nexport const x = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/b.tsx': 'src/a.tsx',
    })
  })

  it('keeps an import of a value binding literally named `type`', () => {
    // `{ type }` imports a VALUE named "type" — not an inline type specifier.
    writeFixture(
      'src/a.tsx',
      "import { type } from './b'\nexport const a = type\n",
    )
    writeFixture('src/b.tsx', 'export const type = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/b.tsx': 'src/a.tsx',
    })
  })

  it('should collapse a shared dep into its only VALUE importer when others import it type-only', () => {
    // `a` imports `shared` at runtime; `b` only `import type`s it. Erasing the
    // type edge leaves `shared` with a single real importer -> it collapses into
    // `a`, which is both correct (b never loads it at runtime) and tighter CSS.
    writeFixture('src/a.tsx', "import './shared'\n")
    writeFixture(
      'src/b.tsx',
      "import type { S } from './shared'\nexport const b = 1\n",
    )
    writeFixture(
      'src/shared.tsx',
      'export type S = number\nexport const s = 1\n',
    )

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/shared.tsx': 'src/a.tsx',
    })
  })

  it('treats a value `export { x } from` as a static import (collapses)', () => {
    writeFixture('src/a.tsx', "export { x } from './b'\n")
    writeFixture('src/b.tsx', 'export const x = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/b.tsx': 'src/a.tsx',
    })
  })

  it('ignores import-like code snippets inside template literals', () => {
    // A docs/codegen file embedding example code in a template literal must
    // NOT create a graph edge: the bundler never loads './b', so counting it
    // would make it a phantom bucket member (the coordinator-stall class).
    writeFixture(
      'src/a.tsx',
      [
        'const snippet = `',
        "import { Box } from './b'",
        "import './b'",
        '`',
        "const escaped = `mid \\` import './b' `",
        "import './c'",
        'export const a = 1',
      ].join('\n'),
    )
    writeFixture('src/b.tsx', 'export const b = 1\n')
    writeFixture('src/c.tsx', 'export const c = 1\n')

    // Only the real import ('./c') collapses; './b' stays edge-free.
    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/c.tsx': 'src/a.tsx',
    })
  })

  it('parses imports while stripping comments and string escapes', () => {
    writeFixture(
      'src/a.tsx',
      [
        'const s = "a\\\\b\\tc" // line comment with import \'./not-real\'',
        '/* block comment',
        ' spanning lines with import "./also-not" */',
        "import './b'",
        'export const a = 1',
      ].join('\n'),
    )
    writeFixture('src/b.tsx', 'export const b = 1\n')

    // Imports inside comments/strings are ignored; only the real one collapses.
    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/b.tsx': 'src/a.tsx',
    })
  })

  it('returns no aliases when tsconfig has no compilerOptions', () => {
    writeFixture('tsconfig.json', '{}')
    writeFixture('src/a.tsx', "import '@/foo'\n")
    writeFixture('src/foo.tsx', 'export const foo = 1\n')

    expect(
      buildCanonicalMap({
        cwd,
        srcDir,
        tsconfigPath: join(cwd, 'tsconfig.json'),
      }),
    ).toEqual({})
  })

  it('returns no aliases when tsconfig compilerOptions has no paths', () => {
    writeFixture(
      'tsconfig.json',
      JSON.stringify({ compilerOptions: { baseUrl: '.' } }),
    )
    writeFixture('src/a.tsx', "import '@/foo'\n")
    writeFixture('src/foo.tsx', 'export const foo = 1\n')

    expect(
      buildCanonicalMap({
        cwd,
        srcDir,
        tsconfigPath: join(cwd, 'tsconfig.json'),
      }),
    ).toEqual({})
  })

  it('ignores a malformed tsconfig (JSON parse error)', () => {
    writeFixture('tsconfig.json', '{ this is not json')
    writeFixture('src/a.tsx', "import './b'\n")
    writeFixture('src/b.tsx', 'export const b = 1\n')

    expect(
      buildCanonicalMap({
        cwd,
        srcDir,
        tsconfigPath: join(cwd, 'tsconfig.json'),
      }),
    ).toEqual({ 'src/b.tsx': 'src/a.tsx' })
  })

  it('prefers the longest-prefix alias when multiple tsconfig paths overlap', () => {
    // Two aliases -> the prefix-length sort comparator runs; the longer prefix
    // (`@components/`) must win over the broader `@/`.
    writeFixture(
      'tsconfig.json',
      JSON.stringify({
        compilerOptions: {
          baseUrl: '.',
          paths: {
            '@/*': ['src/*'],
            '@components/*': ['src/components/*'],
          },
        },
      }),
    )
    writeFixture('src/a.tsx', "import '@components/x'\n")
    writeFixture('src/components/x.tsx', 'export const x = 1\n')

    expect(
      buildCanonicalMap({
        cwd,
        srcDir,
        tsconfigPath: join(cwd, 'tsconfig.json'),
      }),
    ).toEqual({ 'src/components/x.tsx': 'src/a.tsx' })
  })

  it('handles a root-absolute import specifier', () => {
    writeFixture('src/a.tsx', "import '/abs/thing'\n")

    // The `/`-prefixed branch runs; it resolves outside srcDir -> unresolved.
    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({})
  })

  it('resolves an import that includes an explicit .tsx extension', () => {
    writeFixture('src/a.tsx', "import './b.tsx'\n")
    writeFixture('src/b.tsx', 'export const b = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/b.tsx': 'src/a.tsx',
    })
  })

  it('resolves a directory import to its index file', () => {
    writeFixture('src/a.tsx', "import './dir'\n")
    writeFixture('src/dir/index.tsx', 'export const d = 1\n')

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/dir/index.tsx': 'src/a.tsx',
    })
  })

  it('leaves an import that resolves to no file unresolved', () => {
    writeFixture('src/a.tsx', "import './ghost'\n")

    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({})
  })

  it('keys the map by absolute POSIX path when keyBy is "absolute"', () => {
    writeFixture('src/a.tsx', "import './b'\n")
    writeFixture('src/b.tsx', 'export const b = 1\n')

    const map = buildCanonicalMap({ cwd, srcDir, keyBy: 'absolute' })
    const [key, value] = Object.entries(map)[0]

    // Absolute POSIX (backslashes normalized), not cwd-relative.
    expect(key).not.toContain('\\')
    expect(key.endsWith('/src/b.tsx')).toBe(true)
    expect(value.endsWith('/src/a.tsx')).toBe(true)
    expect(key).not.toBe('src/b.tsx')
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

describe('buildStaticImportGraph sharing', () => {
  let tempRoot: string
  let cwd: string
  let srcDir: string

  beforeEach(() => {
    tempRoot = mkdtempSync(join(tmpdir(), 'devup-ui-shared-graph-'))
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

  it('yields identical results whether builders scan or reuse one graph', () => {
    writeFixture('src/app/layout.tsx', "import './shared'\n")
    writeFixture('src/app/shared.tsx', 'export const Shared = 1\n')
    writeFixture('src/app/a/page.tsx', "import './a-only'\n")
    writeFixture('src/app/a/a-only.tsx', 'export const A = 1\n')
    writeFixture('src/app/b/page.tsx', 'export const B = 1\n')
    writeFixture(
      'src/lazy-host.tsx',
      "export const load = () => import('./lazy')\n",
    )
    writeFixture('src/lazy.tsx', 'export const L = 1\n')

    const graph = buildStaticImportGraph(srcDir)

    expect(buildCanonicalMap({ cwd, srcDir, graph })).toEqual(
      buildCanonicalMap({ cwd, srcDir }),
    )
    expect(computeFileRoutes({ cwd, srcDir, graph })).toEqual(
      computeFileRoutes({ cwd, srcDir }),
    )
    expect(computeFileReach({ cwd, srcDir, graph })).toEqual(
      computeFileReach({ cwd, srcDir }),
    )
  })
})

describe('planAtomHoist', () => {
  it('folds reach onto the canonical bucket and skips @global', () => {
    const plan = planAtomHoist(
      { 'src/child.tsx': 'src/parent.tsx', 'src/glob.tsx': '@global' },
      {
        'src/parent.tsx': [0, 1],
        'src/child.tsx': [0],
        'src/glob.tsx': [0, 1],
        'src/r1.tsx': [1],
      },
      2,
    )
    expect(plan).toEqual({
      threshold: 2,
      // child folded into parent (deduped); @global dropped; r1 kept
      reachByBucket: {
        'src/parent.tsx': [0, 1],
        'src/r1.tsx': [1],
      },
    })
  })

  it('clamps the threshold to a minimum of 2', () => {
    const plan = planAtomHoist({}, { 'a.tsx': [0], 'b.tsx': [1] }, 1)
    expect(plan?.threshold).toBe(2)
  })

  it('honors a threshold above 2', () => {
    const plan = planAtomHoist({}, { 'a.tsx': [0], 'b.tsx': [1] }, 5)
    expect(plan?.threshold).toBe(5)
  })

  it('returns null when fewer than two distinct routes exist', () => {
    expect(planAtomHoist({}, { 'a.tsx': [0] }, 2)).toBeNull()
    expect(planAtomHoist({}, {}, 2)).toBeNull()
  })
})

// The oxc AST path is the fast parser used when `oxc-parser` is installed in
// the host project. It is absent in this repo, so we inject a fake parser to
// exercise the AST walk (module state is shared across test files, so this is
// reset after each test back to the regex fallback).
describe('oxc AST parsing path', () => {
  let tempRoot: string
  let cwd: string
  let srcDir: string

  beforeEach(() => {
    tempRoot = mkdtempSync(join(tmpdir(), 'devup-ui-oxc-'))
    cwd = join(tempRoot, 'project')
    srcDir = join(cwd, 'src')
    mkdirSync(srcDir, { recursive: true })
  })

  afterEach(() => {
    __setOxcParserForTest(undefined)
    rmSync(tempRoot, { recursive: true, force: true })
  })

  function writeFixture(path: string, code: string): void {
    const filePath = join(cwd, path)
    mkdirSync(dirname(filePath), { recursive: true })
    writeFileSync(filePath, code)
  }

  it('collects every import/export node kind from the AST (type-only excluded)', () => {
    const circular: Record<string, unknown> = { type: 'SelfRef' }
    circular.self = circular // self-reference -> exercises the `seen` guard
    const richProgram = {
      type: 'Program',
      body: [
        // value import -> static edge (getStringLiteralValue via `.value`)
        {
          type: 'ImportDeclaration',
          importKind: 'value',
          source: { value: './val' },
        },
        // `import type` -> skipped (importKind 'type')
        {
          type: 'ImportDeclaration',
          importKind: 'type',
          source: { value: './t1' },
        },
        // value re-export -> static edge
        {
          type: 'ExportNamedDeclaration',
          exportKind: 'value',
          source: { value: './exp' },
        },
        // `export type` -> skipped (exportKind 'type')
        {
          type: 'ExportNamedDeclaration',
          exportKind: 'type',
          source: { value: './t2' },
        },
        // export-all -> static edge
        { type: 'ExportAllDeclaration', source: { value: './all' } },
        // dynamic import expression with `.source`
        { type: 'ImportExpression', source: { value: './dyn1' } },
        // dynamic import expression falling back to `.argument`
        { type: 'ImportExpression', argument: { value: './dyn2' } },
        // import() call via callee.type === 'Import', specifier via `.raw`
        {
          type: 'CallExpression',
          callee: { type: 'Import' },
          arguments: [{ raw: "'./dyn3'" }],
        },
        // import() call via callee.name === 'import'
        {
          type: 'CallExpression',
          callee: { name: 'import' },
          arguments: [{ value: './dyn4' }],
        },
        // import() with non-array arguments -> first arg undefined -> no push
        {
          type: 'CallExpression',
          callee: { type: 'Import' },
          arguments: 'not-an-array',
        },
        // non-import call (isImportCallee false via name) -> falls through
        { type: 'CallExpression', callee: { name: 'other' }, arguments: [] },
        // non-record callee -> isImportCallee returns false
        { type: 'CallExpression', callee: null, arguments: [] },
        // source literal with neither `.value` nor `.raw` -> no push
        {
          type: 'ImportDeclaration',
          importKind: 'value',
          source: { kind: 'no-literal' },
        },
        circular,
        'primitive-child',
        7,
        null,
      ] as unknown[],
    }
    __setOxcParserForTest({
      parseSync: (filename: string) =>
        filename.endsWith('a.tsx')
          ? { program: richProgram }
          : { program: { type: 'Program', body: [] as unknown[] } },
    })

    writeFixture('src/a.tsx', 'parsed by the fake oxc parser, content ignored')
    writeFixture(
      'src/val.tsx',
      'parsed by the fake oxc parser, content ignored',
    )

    // Proof the AST path ran: `a` statically imports `./val` -> val collapses
    // into a. The regex fallback would parse the literal content -> no imports.
    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/val.tsx': 'src/a.tsx',
    })
  })

  it('skips AST import/export nodes whose specifiers are all inline-type', () => {
    const program = {
      type: 'Program',
      body: [
        // all-inline-type import -> erased by the bundler -> no edge
        {
          type: 'ImportDeclaration',
          importKind: 'value',
          specifiers: [{ type: 'ImportSpecifier', importKind: 'type' }],
          source: { value: './phantom' },
        },
        // all-inline-type re-export -> erased -> no edge
        {
          type: 'ExportNamedDeclaration',
          exportKind: 'value',
          specifiers: [{ type: 'ExportSpecifier', exportKind: 'type' }],
          source: { value: './phantom2' },
        },
        // mixed inline types -> module still imported -> edge kept
        {
          type: 'ImportDeclaration',
          importKind: 'value',
          specifiers: [
            { type: 'ImportSpecifier', importKind: 'type' },
            { type: 'ImportSpecifier', importKind: 'value' },
          ],
          source: { value: './mixed' },
        },
        // default specifier (no importKind) alongside an inline type -> kept
        {
          type: 'ImportDeclaration',
          importKind: 'value',
          specifiers: [
            { type: 'ImportDefaultSpecifier' },
            { type: 'ImportSpecifier', importKind: 'type' },
          ],
          source: { value: './withdefault' },
        },
        // non-record specifier entry -> not type-only -> kept
        {
          type: 'ImportDeclaration',
          importKind: 'value',
          specifiers: ['bogus'],
          source: { value: './bogus' },
        },
        // empty specifier list (`import {} from`) -> kept (side-effect import)
        {
          type: 'ImportDeclaration',
          importKind: 'value',
          specifiers: [] as unknown[],
          source: { value: './empty' },
        },
      ] as unknown[],
    }
    __setOxcParserForTest({
      parseSync: (filename: string) =>
        filename.endsWith('a.tsx')
          ? { program }
          : { program: { type: 'Program', body: [] as unknown[] } },
    })

    writeFixture('src/a.tsx', 'fake parser input')
    writeFixture('src/phantom.tsx', 'fake parser input')
    writeFixture('src/phantom2.tsx', 'fake parser input')
    writeFixture('src/mixed.tsx', 'fake parser input')
    writeFixture('src/withdefault.tsx', 'fake parser input')
    writeFixture('src/bogus.tsx', 'fake parser input')
    writeFixture('src/empty.tsx', 'fake parser input')

    // phantom/phantom2 gain no importer (edges dropped) -> roots, not members.
    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/bogus.tsx': 'src/a.tsx',
      'src/empty.tsx': 'src/a.tsx',
      'src/mixed.tsx': 'src/a.tsx',
      'src/withdefault.tsx': 'src/a.tsx',
    })
  })

  it('falls back to the regex scan when the oxc parser throws', () => {
    __setOxcParserForTest({
      parseSync: () => {
        throw new Error('boom')
      },
    })

    writeFixture('src/a.tsx', "import './b'\n")
    writeFixture('src/b.tsx', 'export const b = 1\n')

    // parseSync throws -> parseImportsWithOxc returns undefined -> scanImports.
    expect(buildCanonicalMap({ cwd, srcDir })).toEqual({
      'src/b.tsx': 'src/a.tsx',
    })
  })
})

describe('runImportGraphCli', () => {
  function makeProject(): { root: string; cwd: string } {
    const root = mkdtempSync(join(tmpdir(), 'devup-ui-cli-'))
    const cwd = join(root, 'project')
    const srcDir = join(cwd, 'src')
    mkdirSync(srcDir, { recursive: true })
    writeFileSync(join(srcDir, 'a.tsx'), "import './b'\n")
    writeFileSync(join(srcDir, 'b.tsx'), 'export const b = 1\n')
    return { root, cwd }
  }

  it('prints usage and exits when the srcDir arg is missing', () => {
    const errorSpy = spyOn(console, 'error').mockReturnValue(undefined)
    const exitSpy = spyOn(process, 'exit').mockImplementation(
      (() => undefined) as never,
    )

    runImportGraphCli([])

    expect(errorSpy).toHaveBeenCalled()
    expect(exitSpy).toHaveBeenCalledWith(1)

    errorSpy.mockRestore()
    exitSpy.mockRestore()
  })

  it('prints the canonical map JSON to stdout when no outFile is given', () => {
    const { root, cwd } = makeProject()
    const infoSpy = spyOn(console, 'info').mockReturnValue(undefined)

    runImportGraphCli(['src', cwd])

    const printed = (infoSpy.mock.calls[0] as [string])[0]
    expect(printed).toContain('src/b.tsx')

    infoSpy.mockRestore()
    rmSync(root, { recursive: true, force: true })
  })

  it('writes the canonical map JSON to outFile (with a tsconfig arg)', () => {
    const { root, cwd } = makeProject()
    writeFileSync(
      join(cwd, 'tsconfig.json'),
      JSON.stringify({ compilerOptions: { baseUrl: '.' } }),
    )

    runImportGraphCli(['src', cwd, 'tsconfig.json', 'out.json'])

    const written = JSON.parse(readFileSync(join(cwd, 'out.json'), 'utf-8'))
    expect(written).toEqual({ 'src/b.tsx': 'src/a.tsx' })

    rmSync(root, { recursive: true, force: true })
  })

  it('defaults cwd to process.cwd() when only srcDir is given', () => {
    const infoSpy = spyOn(console, 'info').mockReturnValue(undefined)

    // A non-existent srcDir -> empty map -> prints "{}" without touching files.
    runImportGraphCli(['__devup_nonexistent_src__'])

    expect(infoSpy).toHaveBeenCalledWith('{}')

    infoSpy.mockRestore()
  })
})
