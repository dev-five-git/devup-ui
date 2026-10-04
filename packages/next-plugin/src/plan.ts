import { join, relative } from 'node:path'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  collectNumberedFiles,
  computeCompiledFiles,
  computeFileRoutes,
  extractedNeedles,
  planAtomHoist,
  type StaticImportGraph,
} from '@devup-ui/plugin-utils'

import { locatedError } from './build-error'
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

function planRoutes(context: AppContext): RoutePlan {
  const startedAt = profileStart()
  const srcDir = join(context.root, 'src')
  const tsconfigPath = join(context.root, 'tsconfig.json')
  const cwd = context.root
  // Root-level `app/` and `pages/` routes join the graph here once
  // buildStaticImportGraph accepts a project root (fix/plugin-core).
  const graph = buildStaticImportGraph(srcDir, tsconfigPath)
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
  const expectedBaseFiles = computeCompiledFiles({
    srcDir,
    tsconfigPath,
    cwd,
    graph,
  })
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
  const routes = recoverPlanning({
    context,
    file: join(context.root, 'src'),
    what: 'devup-ui import graph',
    code: 'buildStaticImportGraph',
    needs: 'readable source files and a parseable tsconfig.json',
    lost: 'single-importer collapse, atom hoisting and the deterministic completion set',
    fallback: UNPLANNED,
    work: () => planRoutes(context),
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
        roots: [...context.sourceRoots],
        include: [...context.include],
        cwd: context.root,
        needles: extractedNeedles(context.libPackage, context.importAliases),
        toId: (path) => relative(context.root, path).replaceAll('\\', '/'),
      }),
  })
  return { ...routes, seedFiles }
}
