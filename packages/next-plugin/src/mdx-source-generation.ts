import { join } from 'node:path'
import { isDeepStrictEqual } from 'node:util'

import {
  computeReachableFiles,
  type StaticImportGraph,
} from '@devup-ui/plugin-utils'

import type { PreparedSourceGeneration } from './coordinator-options'
import { affectedMdxSources } from './mdx-source-affected'
import {
  exportMdxRestartCache,
  importMdxRestartCache,
  type MdxCacheEntry,
} from './mdx-source-cache'
import { withMdxSourceControl } from './mdx-source-control'
import { deliverMdxSourceGeneration } from './mdx-source-delivery'
import {
  changedMdxInput,
  MdxFreshnessError,
  type MdxInputFingerprint,
} from './mdx-source-freshness'
import { collectMdxOrdinaryInputs } from './mdx-source-inputs'
import { MdxNativeInputPendingError } from './mdx-source-pending'
import { runMdxSourcePreparation } from './mdx-source-run'
import type {
  MdxBuildBinding,
  MdxNativeExpectation,
  MdxPreparationRun,
  MdxSourceGeneration,
} from './mdx-source-types'

export { affectedMdxSources } from './mdx-source-affected'
export type {
  MdxBuildBinding,
  MdxPipelineSelection,
  MdxSourceGeneration,
} from './mdx-source-types'

export class MdxSourceExtensionError extends Error {
  readonly name = 'MdxSourceExtensionError'
  constructor(
    readonly configFile: string,
    readonly extension: string,
  ) {
    super(
      `${configFile}:1:1: MDX extension predicate ${extension} needs a certified finite anchored literal suffix condition; use a literal extension or finite extension alternation`,
    )
  }
}

export function createMdxSourceManager(
  binding: MdxBuildBinding,
  restartCache?: string,
) {
  const root = binding.effectiveAppContext.root
  const stored =
    restartCache === undefined
      ? new Map<string, MdxCacheEntry>()
      : importMdxRestartCache(restartCache)
  const states = new WeakMap<
    PreparedSourceGeneration['configureWasm'],
    {
      readonly generation: MdxSourceGeneration
      readonly graph: StaticImportGraph
      readonly entries: ReadonlyMap<string, MdxCacheEntry>
      readonly ordinary: readonly MdxInputFingerprint[]
      readonly pending: ReadonlyMap<string, MdxNativeExpectation>
    }
  >()
  async function prepare(
    signal: AbortSignal,
    previous?: MdxSourceGeneration,
    control: MdxPreparationRun = {},
  ): Promise<MdxSourceGeneration> {
    return withMdxSourceControl(
      { filename: root, signal },
      async (signal, deadline) => {
        const old = previous ? stateFor(previous) : undefined
        const changes = control.changedPaths ?? []
        const dirty = new Set(changes)
        if (previous && old) {
          for (const input of old.ordinary)
            if (changedMdxInput(root, [input])) dirty.add(input.path)
          for (const [filename, entry] of old.entries)
            if (changedMdxInput(filename, entry.inputs)) dirty.add(filename)
          for (const path of changes)
            for (const filename of affectedMdxSources(previous, path))
              dirty.add(filename)
          for (const filename of old.entries.keys()) {
            if (
              computeReachableFiles({
                srcDir: [...binding.effectiveAppContext.sourceRoots, root],
                tsconfigPath: join(root, 'tsconfig.json'),
                graph: old.graph,
                entries: [filename],
              }).some((path) => dirty.has(path))
            )
              dirty.add(filename)
          }
        }
        const run = await runMdxSourcePreparation(binding, {
          signal,
          candidates: old?.entries ?? stored,
          dirty,
          deadline,
        })
        if (
          previous &&
          old &&
          run.entries.size === old.entries.size &&
          [...run.entries].every(
            ([filename, entry]) => entry === old.entries.get(filename),
          ) &&
          isDeepStrictEqual(
            [
              run.plan.seedFiles,
              run.plan.canonicalMap,
              run.plan.fileRoutes,
              run.plan.atomThreshold,
              run.plan.expectedBaseFiles,
            ],
            [
              previous.plan.seedFiles,
              previous.plan.canonicalMap,
              previous.plan.fileRoutes,
              previous.plan.atomThreshold,
              previous.plan.expectedBaseFiles,
            ],
          )
        ) {
          const ordinary = collectMdxOrdinaryInputs(binding, [
            ...run.planned,
            ...old.ordinary.map((input) => input.path),
          ])
          if (
            ordinary.pending.length === 0 &&
            !changedMdxInput(root, old.ordinary) &&
            JSON.stringify(
              ordinary.ordinaryInputs.map((input) => [
                input.filename,
                input.source,
              ]),
            ) ===
              JSON.stringify(
                previous.ordinaryInputs.map((input) => [
                  input.filename,
                  input.source,
                ]),
              )
          )
            return previous
        }
        const delivered = await deliverMdxSourceGeneration(
          control.extractDependencies === undefined
            ? binding
            : { ...binding, extractDependencies: control.extractDependencies },
          run,
          signal,
        )
        states.set(delivered.generation.configureWasm, {
          generation: delivered.generation,
          graph: run.graph,
          entries: run.entries,
          ordinary: delivered.ordinary,
          pending: run.pending,
        })
        return delivered.generation
      },
    )
  }
  function stateFor(generation: PreparedSourceGeneration) {
    const state = states.get(generation.configureWasm)
    if (
      !state ||
      !isDeepStrictEqual(state.generation.sources, generation.sources)
    )
      throw new TypeError('Generation does not belong to this source manager')
    return state
  }
  type RefreshRequest<T extends PreparedSourceGeneration> =
    MdxPreparationRun & {
      readonly generation: T
      readonly signal: AbortSignal
      readonly changedPaths?: readonly string[]
    }
  function refresh(
    request: RefreshRequest<MdxSourceGeneration>,
  ): Promise<MdxSourceGeneration>
  function refresh(
    request: RefreshRequest<PreparedSourceGeneration>,
  ): Promise<PreparedSourceGeneration>
  async function refresh(
    request: RefreshRequest<PreparedSourceGeneration>,
  ): Promise<PreparedSourceGeneration> {
    const original = stateFor(request.generation).generation
    const next = await prepare(request.signal, original, request)
    return next === original ? request.generation : next
  }
  return Object.freeze({
    prepare: (signal: AbortSignal, control?: MdxPreparationRun) =>
      prepare(signal, undefined, control),
    refresh,
    validateForCssFinalization(generation: PreparedSourceGeneration) {
      const state = stateFor(generation)
      const pending = state.pending.values().next().value
      if (pending) throw new MdxNativeInputPendingError(pending)
      for (const [filename, entry] of state.entries) {
        const changed = changedMdxInput(filename, entry.inputs)
        if (changed)
          throw new MdxFreshnessError(
            filename,
            changed,
            'input changed before CSS finalization',
          )
      }
      const changed = changedMdxInput(root, state.ordinary)
      if (changed)
        throw new MdxFreshnessError(
          root,
          changed,
          'ordinary input changed before CSS finalization',
        )
    },
    restartCache: (generation: PreparedSourceGeneration) =>
      exportMdxRestartCache(stateFor(generation).entries),
  })
}
