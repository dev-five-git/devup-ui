import { resolve } from 'node:path'

import { computeReachableFiles } from '@devup-ui/plugin-utils'

import type { MdxCacheEntry } from './mdx-source-cache'
import {
  changedMdxInput,
  fingerprintMdxInput,
  MdxFreshnessError,
} from './mdx-source-freshness'
import { immutableMdxPlan } from './mdx-source-immutable'
import {
  collectMdxOrdinaryInputs,
  ordinaryExtractionEvidence,
  sourceInputKey,
} from './mdx-source-inputs'
import type { runMdxSourcePreparation } from './mdx-source-run'
import type {
  MdxBuildBinding,
  MdxExtractionReport,
  MdxExtractionView,
  MdxSourceGeneration,
} from './mdx-source-types'
import type { DevupWasm } from './wasm'

function compiledSources(
  root: string,
  entries: ReadonlyMap<string, MdxCacheEntry>,
) {
  return [...entries].map(([filename, entry]) =>
    Object.freeze({
      input: Object.freeze({
        filename: sourceInputKey(root, filename),
        resourcePath: filename,
        source: entry.prepared.source,
        sourceType: 'compiled-mdx' as const,
        dependencies: Object.freeze(
          entry.inputs
            .filter(
              (input) =>
                (input.kind === 'file' || input.kind === 'build') &&
                input.path !== filename,
            )
            .map((input) => sourceInputKey(root, input.path)),
        ),
        stamps: Object.freeze(
          Object.fromEntries(
            entry.inputs
              .filter(
                (input) => input.kind === 'file' || input.kind === 'build',
              )
              .map((input) => [input.path, input.fingerprint]),
          ),
        ),
        backing:
          entry.inputs.find((input) => input.path === filename)?.fingerprint ??
          '',
      }),
      evidence: Object.freeze({
        compilerFingerprint: entry.compilerKey,
        fileFingerprints: Object.freeze(
          Object.fromEntries(
            entry.inputs
              .filter(
                (input) => input.kind === 'file' || input.kind === 'build',
              )
              .map((input) => [input.path, input.fingerprint]),
          ),
        ),
        contextFingerprints: Object.freeze(
          Object.fromEntries(
            entry.inputs
              .filter((input) => input.kind === 'context')
              .map((input) => [input.path, input.fingerprint]),
          ),
        ),
        missingDependencies: entry.prepared.missingDependencies,
        ...(typeof entry.prepared.map === 'string'
          ? { map: entry.prepared.map }
          : {}),
      }),
    }),
  )
}

export async function deliverMdxSourceGeneration(
  binding: MdxBuildBinding,
  run: Awaited<ReturnType<typeof runMdxSourcePreparation>>,
  signal: AbortSignal,
) {
  const { entries, planned, resolver, cacheReader } = run
  const root = binding.effectiveAppContext.root
  const plan = immutableMdxPlan(run.plan, run.graph)
  let ordinary = collectMdxOrdinaryInputs(binding, [...planned])
  run.enableExtraction()
  let reports: readonly MdxExtractionReport[] = []
  if (ordinary.pending.length === 0) {
    for (;;) {
      const view: MdxExtractionView = {
        plan,
        resolver,
        inputs: Object.freeze([
          ...ordinary.ordinaryInputs,
          ...compiledSources(root, entries).map((source) => source.input),
        ]),
      }
      try {
        reports = await binding.extractDependencies(view, signal)
      } catch (cause) {
        if (run.pending.size === 0) throw cause
        if (!(cause instanceof Error)) throw cause
        break
      }
      signal.throwIfAborted()
      const dependencies = reports
        .flatMap((report) => report.dependencies)
        .map((file) => resolve(root, file))
      const newFiles = dependencies.filter((file) => !planned.has(file))
      for (const file of newFiles) planned.add(file)
      if (newFiles.length === 0) break
      ordinary = collectMdxOrdinaryInputs(binding, [...planned])
      if (ordinary.pending.length) break
    }
  }
  for (const expectation of ordinary.pending)
    run.pending.set(expectation.filename, expectation)
  for (const report of reports) {
    const filename = resolve(root, report.filename)
    const entry = entries.get(filename)
    if (!entry) continue
    const inputs = new Map(
      entry.inputs.map((input) => [`${input.kind}:${input.path}`, input]),
    )
    for (const dependency of report.dependencies) {
      const path = resolve(root, dependency)
      if (inputs.has(`file:${path}`)) continue
      inputs.set(
        `file:${path}`,
        fingerprintMdxInput(filename, {
          kind: 'file',
          path,
          loader: 'devup/extractor',
        }),
      )
    }
    entries.set(
      filename,
      Object.freeze({ ...entry, inputs: Object.freeze([...inputs.values()]) }),
    )
  }
  for (const [filename, entry] of entries) {
    const reached = new Set(
      computeReachableFiles({
        srcDir: [...binding.effectiveAppContext.sourceRoots, root],
        graph: run.graph,
        entries: [filename],
      }),
    )
    const inputs = new Map(
      entry.inputs.map((input) => [`${input.kind}:${input.path}`, input]),
    )
    for (const input of ordinary.fingerprints)
      if (reached.has(input.path) && !inputs.has(`file:${input.path}`))
        inputs.set(`file:${input.path}`, input)
    entries.set(
      filename,
      Object.freeze({ ...entry, inputs: Object.freeze([...inputs.values()]) }),
    )
  }
  for (const [filename, entry] of entries) {
    const changed = changedMdxInput(filename, entry.inputs)
    if (changed)
      throw new MdxFreshnessError(
        filename,
        changed,
        'input changed after preparation',
      )
  }
  const ordinaryChanged = changedMdxInput(root, ordinary.fingerprints)
  if (ordinaryChanged)
    throw new MdxFreshnessError(
      root,
      ordinaryChanged,
      'ordinary input changed during extraction',
    )
  signal.throwIfAborted()
  const ordinaryInputs = ordinaryExtractionEvidence(ordinary.ordinaryInputs, {
    root,
    reports,
    fingerprints: [
      ...ordinary.fingerprints,
      ...[...entries.values()].flatMap((entry) => entry.inputs),
    ],
  })
  const inputs = Object.freeze(
    [
      ...ordinaryInputs,
      ...compiledSources(root, entries).map((source) => source.input),
    ].sort((a, b) => a.filename.localeCompare(b.filename)),
  )
  const view = Object.freeze({ plan, resolver, inputs })
  const generation: MdxSourceGeneration = Object.freeze({
    ...view,
    key: Object.freeze({}),
    compiled: Object.freeze(
      Object.fromEntries(
        [...entries].map(([filename, entry]) => [
          filename,
          Object.freeze({ prepared: entry.prepared, inputs: entry.inputs }),
        ]),
      ),
    ),
    sources: Object.freeze(compiledSources(root, entries)),
    ordinaryInputs,
    pendingOrdinary: Object.freeze([...run.pending.values()]),
    cacheReader,
    watchInputs: Object.freeze(
      [
        ...new Set([
          ...ordinary.fingerprints.map((input) => input.path),
          ...run.pending.keys(),
          ...[...entries.values()].flatMap((entry) =>
            entry.inputs.map((input) => input.path),
          ),
        ]),
      ].sort(),
    ),
    extractionDependencies: Object.freeze(
      reports.map((report) =>
        Object.freeze({
          ...report,
          dependencies: Object.freeze([...report.dependencies]),
        }),
      ),
    ),
    configureWasm: (wasm: DevupWasm) => binding.configureWasm(wasm, view),
  })
  return { generation, ordinary: ordinary.fingerprints }
}
