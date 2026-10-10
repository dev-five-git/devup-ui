import { join, resolve } from 'node:path'

import {
  buildStaticImportGraph,
  computeReachableFiles,
  createModuleResolver,
} from '@devup-ui/plugin-utils'

import { requireMdxPipeline } from './mdx-pipeline'
import {
  compileMdx,
  createMdxOptionsInstance,
  type MdxDeadline,
} from './mdx-prepare'
import { createResolutionProof } from './mdx-resolution-proof'
import {
  captureMdxCacheIdentity,
  compatibleMdxCache,
  type MdxCacheEntry,
} from './mdx-source-cache'
import {
  captureMdxFreshness,
  changedMdxInput,
  createMdxTimestampAccuracy,
  MdxFreshnessError,
} from './mdx-source-freshness'
import { immutableMdxMap } from './mdx-source-immutable'
import { MdxNativeInputPendingError } from './mdx-source-pending'
import type { MdxBuildBinding, MdxNativeExpectation } from './mdx-source-types'
import {
  planPreparedSources,
  planSourceGraph,
  sourceDiscoveryExcludes,
} from './plan'
import { collectPrewarmFiles, EXTRACTABLE_EXTENSION } from './prewarm'

export async function runMdxSourcePreparation(
  binding: MdxBuildBinding,
  control: {
    readonly signal: AbortSignal
    readonly candidates: ReadonlyMap<string, MdxCacheEntry>
    readonly dirty: ReadonlySet<string>
    readonly deadline: MdxDeadline
  },
) {
  const { signal, candidates, dirty, deadline } = control
  signal.throwIfAborted()
  const context = binding.effectiveAppContext
  const roots = [...context.sourceRoots, context.root]
  const tsconfig = join(context.root, 'tsconfig.json')
  const optionsInstance = createMdxOptionsInstance()
  const accuracy = createMdxTimestampAccuracy()
  const entries = new Map<string, MdxCacheEntry>()
  const pending = new Map<string, MdxNativeExpectation>()
  const resolution = createResolutionProof(context.root)
  let extracting = false
  const cacheReader = (filename: string) => {
    const prepared = entries.get(resolve(filename))?.prepared
    if (!prepared && extracting && EXTRACTABLE_EXTENSION.test(filename)) {
      const eligibility = binding.ordinaryEligibility(filename)
      switch (eligibility.kind) {
        case 'native-required':
          pending.set(
            filename,
            Object.freeze({ ...eligibility.expectation, filename }),
          )
          throw new MdxNativeInputPendingError(eligibility.expectation)
        case 'disk-first':
          break
        default:
          eligibility satisfies never
      }
    }
    return prepared
      ? {
          code: prepared.source,
          sourceType: 'compiled-mdx' as const,
          ...(prepared.map === undefined ? {} : { map: prepared.map }),
        }
      : undefined
  }
  const resolver = Object.freeze({
    prepareSource: cacheReader,
    alias: binding.aliases,
    conditions: binding.conditions,
    includeMdx: binding.extensions,
    onResolutionInputs: resolution.observe,
  })
  const raw = buildStaticImportGraph(roots, tsconfig, {
    cwd: context.root,
    include: context.include,
    exclude: sourceDiscoveryExcludes(context),
    alias: binding.aliases,
    conditions: binding.conditions,
    includeMdx: binding.extensions,
    onResolutionInputs: resolution.observe,
  })
  let plan = { ...planSourceGraph(context, raw), seedFiles: [] as string[] }
  for (;;) {
    signal.throwIfAborted()
    if (Date.now() >= deadline.expiresAt)
      throw new MdxFreshnessError(
        context.root,
        { kind: 'file', path: context.root, loader: 'generation' },
        'preparation deadline exceeded',
      )
    const graph = plan.graph ?? raw
    const files = collectPrewarmFiles({
      root: context.root,
      graph,
      expectedBaseFiles: plan.expectedBaseFiles,
      libPackage: context.libPackage,
      include: context.include,
      prewarmAll: context.prewarmAll,
      resolver,
    })
    const reached = files
      .map((file) => resolve(context.root, file))
      .filter((filename) =>
        binding.extensions.some((extension) =>
          filename.toLowerCase().endsWith(extension),
        ),
      )
    let added = false
    for (const filename of reached) {
      if (entries.has(filename)) continue
      const selection = await binding.selectPipeline(filename, signal)
      signal.throwIfAborted()
      if (!selection) continue
      const cached = candidates.get(filename)
      const dependencyChanged =
        dirty.has(filename) ||
        computeReachableFiles({
          srcDir: roots,
          tsconfigPath: tsconfig,
          graph,
          entries: [filename],
        }).some((path) => dirty.has(path))
      if (
        cached &&
        !dependencyChanged &&
        compatibleMdxCache(cached, selection) &&
        !changedMdxInput(filename, cached.inputs) &&
        !changedMdxInput(filename, cached.resolutionInputs ?? [])
      )
        entries.set(filename, cached)
      else {
        const identity = captureMdxCacheIdentity(selection)
        const startedAt = Date.now()
        const prepared = await compileMdx({
          root: context.root,
          filename,
          pipeline: requireMdxPipeline(filename, selection.pipeline),
          context: { ...selection.context, generation: optionsInstance },
          signal,
          deadline,
          optionsInstance,
        })
        signal.throwIfAborted()
        const inputs = captureMdxFreshness(prepared, {
          startedAt,
          development: context.watch,
          accuracy,
        })
        entries.set(
          filename,
          Object.freeze({
            ...identity,
            prepared: Object.freeze({
              ...prepared,
              map: immutableMdxMap(prepared.map),
              dependencies: Object.freeze([...prepared.dependencies]),
              contextDependencies: Object.freeze([
                ...prepared.contextDependencies,
              ]),
              missingDependencies: Object.freeze([
                ...prepared.missingDependencies,
              ]),
              buildDependencies: Object.freeze([...prepared.buildDependencies]),
              dependencyReports: Object.freeze([...prepared.dependencyReports]),
            }),
            inputs,
          }),
        )
      }
      added = true
    }
    plan = await planPreparedSources(context, {
      extensions: binding.extensions,
      aliases: binding.aliases,
      conditions: binding.conditions,
      cacheReader,
      onResolutionInputs: resolution.observe,
    })
    if (!added) break
  }
  const graph = plan.graph ?? raw
  const files = collectPrewarmFiles({
    root: context.root,
    graph,
    expectedBaseFiles: plan.expectedBaseFiles,
    libPackage: context.libPackage,
    include: context.include,
    prewarmAll: context.prewarmAll,
    resolver,
  })
  const planned = new Set(files.map((file) => resolve(context.root, file)))
  for (const filename of entries.keys())
    if (!planned.has(filename)) entries.delete(filename)
  const verify = createModuleResolver({ cwd: context.root, ...resolver })
  for (const filename of planned) {
    if (
      binding.extensions.some((extension) =>
        filename.toLowerCase().endsWith(extension),
      ) &&
      !entries.has(filename)
    )
      verify(
        filename,
        [...(graph.staticImporters.get(filename) ?? [])][0] ?? filename,
      )
  }
  return {
    entries,
    cacheReader,
    resolver,
    graph,
    plan,
    planned,
    pending,
    resolution,
    enableExtraction: () => {
      extracting = true
    },
  }
}
