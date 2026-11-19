import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'

import { computeFileReach, type StaticImportGraph } from '../import-graph'
import { withRouteShells } from '../route-shells'

const root = join(import.meta.dir, 'route-shells-root')
const at = (path: string) => join(root, path)

function graphOf(edges: Record<string, string[]>): StaticImportGraph {
  const files = [
    ...new Set([...Object.keys(edges), ...Object.values(edges).flat()]),
  ].map(at)
  const staticImports = new Map(files.map((file) => [file, new Set<string>()]))
  const staticImporters = new Map(
    files.map((file) => [file, new Set<string>()]),
  )
  for (const [from, targets] of Object.entries(edges))
    for (const target of targets) {
      staticImports.get(at(from))!.add(at(target))
      staticImporters.get(at(target))!.add(at(from))
    }
  return {
    files,
    fileSet: new Set(files),
    staticImports,
    staticImporters,
    dynamicImports: new Map(files.map((file) => [file, new Set<string>()])),
    dynamicTargets: new Set(),
  }
}

/** How many routes reach each file, as atom hoisting counts them */
function reachCounts(graph: StaticImportGraph, routes: string[]) {
  return Object.fromEntries(
    Object.entries(
      computeFileReach({
        srcDir: root,
        cwd: root,
        keyBy: 'cwd-relative',
        graph,
        entries: routes,
      }),
    ).map(([file, reach]) => [file, reach.length]),
  )
}

describe('withRouteShells', () => {
  it('counts what a root layout imports as reached by every route it wraps', () => {
    // Given: Next's entries, the layout among them
    const graph = graphOf({
      'src/app/layout.tsx': ['src/components/Header.tsx'],
      'src/app/page.tsx': [],
      'src/app/a/page.tsx': [],
      'src/app/not-found.tsx': [],
    })
    const routes = [
      'src/app/layout.tsx',
      'src/app/page.tsx',
      'src/app/a/page.tsx',
      'src/app/not-found.tsx',
    ].map(at)

    // When
    const reach = withRouteShells(graph, routes, root)

    // Then: the layout is no route; the three routes below it reach it
    expect(reach.routes).toEqual(routes.slice(1))
    expect(reachCounts(reach.graph, reach.routes)).toMatchObject({
      'src/app/layout.tsx': 3,
      'src/components/Header.tsx': 3,
    })
    // As a route of its own, the header counted once
    expect(reachCounts(graph, routes)['src/components/Header.tsx']).toBe(1)
    // The input graph is left as it was
    expect(graph.staticImports.get(at('src/app/page.tsx'))?.size).toBe(0)
  })

  it('keeps a nested layout or template that wraps only some routes a route of its own', () => {
    const graph = graphOf({
      'app/layout.tsx': [],
      'app/(shop)/template.tsx': ['components/Cart.tsx'],
      'app/(shop)/cart/page.tsx': [],
      'app/(shop)/checkout/loading.tsx': [],
      'app/about/page.tsx': [],
    })
    const routes = [
      'app/layout.tsx',
      'app/(shop)/template.tsx',
      'app/(shop)/cart/page.tsx',
      'app/(shop)/checkout/loading.tsx',
      'app/about/page.tsx',
    ].map(at)

    const reach = withRouteShells(graph, routes, root)

    // The root layout wraps every route; the template only the shop's, so its
    // styles stay off the about page
    expect(reach.routes).toContain(at('app/(shop)/template.tsx'))
    expect(reach.routes).not.toContain(at('app/layout.tsx'))
    expect(reachCounts(reach.graph, reach.routes)).toMatchObject({
      'app/layout.tsx': 3,
      'app/(shop)/template.tsx': 1,
      'components/Cart.tsx': 1,
    })
  })

  it('wraps every Pages Router page in _app, not the API routes or _document', () => {
    const graph = graphOf({
      'pages/_app.tsx': ['components/Nav.tsx'],
      'pages/index.tsx': [],
      'pages/blog/[slug].tsx': [],
      'pages/api/hello.ts': [],
      'pages/_document.tsx': [],
    })
    const routes = [
      'pages/_app.tsx',
      'pages/index.tsx',
      'pages/blog/[slug].tsx',
      'pages/api/hello.ts',
      'pages/_document.tsx',
    ].map(at)

    const reach = withRouteShells(graph, routes, root)

    expect(reachCounts(reach.graph, reach.routes)).toMatchObject({
      'components/Nav.tsx': 2,
      'pages/api/hello.ts': 1,
      'pages/_document.tsx': 1,
    })
  })

  it('keeps routes no shell wraps as they are', () => {
    // Given: a lazily loaded component and an entry outside the app directory
    const graph = graphOf({
      'app/layout.tsx': ['components/Header.tsx'],
      'app/page.tsx': [],
      'app/Chart.tsx': [],
      'src/worker.ts': [],
    })
    const routes = [
      'app/layout.tsx',
      'app/page.tsx',
      'app/Chart.tsx',
      'src/worker.ts',
    ].map(at)

    const reach = withRouteShells(graph, routes, root)

    expect(reach.graph.staticImports.get(at('app/Chart.tsx'))?.size).toBe(0)
    expect(reach.graph.staticImports.get(at('src/worker.ts'))?.size).toBe(0)
    expect(reachCounts(reach.graph, reach.routes)).toMatchObject({
      'components/Header.tsx': 1,
    })
  })

  it('keeps a layout no route sits below a route of its own', () => {
    const graph = graphOf({
      'app/layout.tsx': [],
      'app/page.tsx': [],
      'app/empty/layout.tsx': ['components/Unused.tsx'],
    })
    const routes = [
      'app/layout.tsx',
      'app/page.tsx',
      'app/empty/layout.tsx',
    ].map(at)

    const reach = withRouteShells(graph, routes, root)

    expect(reach.routes).toEqual([
      at('app/page.tsx'),
      at('app/empty/layout.tsx'),
    ])
  })

  it('adds an edge for a route the graph lists no imports of', () => {
    const graph = graphOf({ 'app/layout.tsx': [] })
    const route = at('app/page.tsx')

    const reach = withRouteShells(graph, [at('app/layout.tsx'), route], root)

    expect([...reach.graph.staticImports.get(route)!]).toEqual([
      at('app/layout.tsx'),
    ])
  })

  it('returns the graph untouched without a shell', () => {
    const graph = graphOf({ 'src/a.ts': [], 'src/b.ts': [] })
    const routes = [at('src/a.ts'), at('src/b.ts')]

    const reach = withRouteShells(graph, routes, root)

    expect(reach.graph).toBe(graph)
    expect(reach.routes).toEqual(routes)
  })
})
