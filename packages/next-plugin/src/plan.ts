import { join, relative, resolve } from 'node:path'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  collectNumberedFiles,
  computeFileRoutes,
  computeReachableFiles,
  extractedNeedles,
  planAtomHoist,
  type StaticImportGraph,
} from '@devup-ui/plugin-utils'

import { locatedError } from './build-error'
import { collectNextEntries } from './entries'
import { elapsedMs, profileStart, reportProfile } from './profile'
import type { AppContext } from './session'

/** What the build knows about its sources before anything is extracted. */
export interface SourcePlan {
  readonly graph: StaticImportGraph | undefined
  readonly canonicalMap: Record<string, string>
  /** cwd-relative POSIX files the bundler compiles; the completion signal */
  readonly expectedBaseFiles: string[]
  /** canonical bucket -> route ids, only when atom hoisting is in effect */
  readonly fileRoutes: Record<string, number[]>
  readonly atomThreshold: number | null
  /** every file that may take a number, in the names the plugin extracts */
  readonly seedFiles: string[]
}

type RoutePlan = Omit<SourcePlan, 'seedFiles'>

const UNPLANNED: RoutePlan = {
  graph: undefined,
  canonicalMap: {},
  expectedBaseFiles: [],
  fileRoutes: {},
  atomThreshold: null,
}

interface RecoverFields<T> {
  context: AppContext
  file: string
  what: string
  code: string
  needs: string
  /** What stops being guaranteed when the work fails */
  lost: string
  fallback: T
  work: () => T
}

/**
 * Run planning work whose failure takes a guarantee away. A production build
 * fails with a located error; development warns once, naming what is lost, and
 * continues with `fallback`.
 */
export function recoverPlanning<T>({
  context,
  file,
  what,
  code,
  needs,
  lost,
  fallback,
  work,
}: RecoverFields<T>): T {
  try {
    return work()
  } catch (cause) {
    const error = locatedError({ file, what, code, needs, cause })
    if (context.phase === 'production') throw error
    console.warn(
      `[devup-ui] ${error.message}. Not guaranteed for this session: ${lost}.`,
    )
    return fallback
  }
}

/** Plan a shared graph; prepared compiler inputs can use this same seam later. */
export function planSourceGraph(
  context: AppContext,
  graph: StaticImportGraph,
): RoutePlan {
  const startedAt = profileStart()
  const srcDir = [...context.sourceRoots, context.root]
  const tsconfigPath = join(context.root, 'tsconfig.json')
  const cwd = context.root
  const canonicalMap = buildCanonicalMap({
    srcDir,
    tsconfigPath,
    cwd,
    // Atom hoisting owns the shared-chunk decision, so collapse runs without
    // the file-level @global hoist in atom mode.
    hoistV: context.hoistV,
    graph,
  })
  // Includes files only a dynamic import() reaches: the base sheet must wait
  // for them too.
  const expectedBaseFiles = computeReachableFiles({
    srcDir,
    tsconfigPath,
    entries: collectNextEntries({
      root: cwd,
      files: graph.files,
      pageExtensions: context.pageExtensions,
    }),
    graph,
  })
    .map((file) => relative(cwd, file).replaceAll('\\', '/'))
    .sort()
  const hoist =
    context.atomHoist === undefined
      ? null
      : planAtomHoist(
          canonicalMap,
          computeFileRoutes({ srcDir, tsconfigPath, cwd, graph }),
          context.atomHoist,
        )
  if (context.atomHoist !== undefined && !hoist) {
    console.info(
      '[devup-ui] atomHoist is set but fewer than 2 routes were detected; atom hoisting is a no-op.',
    )
  }
  reportProfile('next.graph', {
    durationMs: elapsedMs(startedAt),
    files: graph.files.length,
    expectedBaseFiles: expectedBaseFiles.length,
  })
  return {
    graph,
    canonicalMap,
    expectedBaseFiles,
    fileRoutes: hoist?.reachByBucket ?? {},
    atomThreshold: hoist?.threshold ?? null,
  }
}

/** Plan the graph, the canonical buckets, atom hoisting and the numbering. */
export function planSources(context: AppContext): SourcePlan {
  const exclude = ['.git', context.nextDistDir, context.distDir]
  const routes = recoverPlanning({
    context,
    file: join(context.root, 'src'),
    what: 'devup-ui import graph',
    code: 'buildStaticImportGraph',
    needs: 'readable source files and a parseable tsconfig.json',
    lost: 'single-importer collapse, atom hoisting and the deterministic completion set',
    fallback: UNPLANNED,
    work: () =>
      planSourceGraph(
        context,
        buildStaticImportGraph(
          [...context.sourceRoots, context.root],
          join(context.root, 'tsconfig.json'),
          {
            cwd: context.root,
            include: context.include,
            exclude,
          },
        ),
      ),
  })
  const seedFiles = recoverPlanning({
    context,
    file: context.root,
    what: 'devup-ui class numbering',
    code: 'collectNumberedFiles',
    needs: 'readable source directories and included packages',
    lost: 'path-ordered class and file numbers',
    fallback: [],
    work: () =>
      collectNumberedFiles({
        roots: [...context.sourceRoots, context.root],
        include: [...context.include],
        cwd: context.root,
        exclude,
        needles: extractedNeedles(context.libPackage, context.importAliases),
        toId: (path) => relative(context.root, path).replaceAll('\\', '/'),
      }).filter(
        (file) =>
          !routes.graph ||
          routes.graph.fileSet.has(resolve(context.root, file)) ||
          file.split('/').includes('node_modules'),
      ),
  })
  return { ...routes, seedFiles }
}
