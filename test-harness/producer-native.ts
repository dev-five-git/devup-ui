import { CoverageError, type SourceCoverage } from './lcov'
import type { BasicBlock, ScriptCapture } from './producer-data'
import { decodeMap, mappedLines } from './producer-map'

export interface NativeFunction {
  readonly startOffset: number
  readonly endOffset: number
  readonly hasExecuted: boolean
}
export interface NativeView {
  readonly code: string
  readonly map: string
  readonly input: string
  readonly functions: readonly NativeFunction[]
  readonly lines: ReadonlyMap<number, number>
  readonly blockLines: ReadonlySet<number>
  readonly coarse: ReadonlyMap<number, readonly NativeFunction[]>
}

export function reconstructNative(
  capture: ScriptCapture,
  source?: SourceCoverage,
  reloads: readonly ScriptCapture[] = [],
): NativeView {
  if (!capture.profile) {
    if (!source)
      throw new CoverageError(
        'missing native observation for unframed public blocks',
      )
    const candidates = reloadUnion(capture, reloads).filter((view) =>
      matchesNative(view, source),
    )
    for (let split = 1; split < capture.blocks.length; split++) {
      const functions = capture.blocks.slice(split)
      if (functions.some((fn) => fn.count !== Number(fn.hasExecuted))) continue
      const view = projectNative(
        capture,
        functions,
        capture.blocks.slice(0, split),
      )
      if (matchesNative(view, source)) candidates.push(view)
    }
    const unique = new Map(
      candidates.map((view) => [
        JSON.stringify({ functions: view.functions, coarse: [...view.coarse] }),
        view,
      ]),
    )
    const candidate = [...unique.values()][0]
    if (unique.size !== 1 || !candidate)
      throw new CoverageError(
        'ambiguous unframed public native function inventory',
      )
    return candidate
  }
  const [synthetic, ...profiles] = capture.profile.functions
  const whole = synthetic?.ranges[0]
  if (
    !whole ||
    whole.startOffset !== 0 ||
    !synthetic.isBlockCoverage ||
    profiles.length === 0
  )
    throw new CoverageError('unsupported precise whole-script profile')
  const functions = capture.blocks.slice(-profiles.length)
  if (whole.endOffset !== capture.code.length)
    throw new CoverageError('contradictory executed source length')
  const identities = profiles
    .map((fn) => {
      const range = fn.ranges[0]
      if (!range) throw new CoverageError('missing precise function identity')
      return `${range.startOffset},${range.endOffset},${fn.isBlockCoverage}`
    })
    .sort()
  const actual = functions
    .map((fn) => `${fn.startOffset},${fn.endOffset},${fn.hasExecuted}`)
    .sort()
  if (
    JSON.stringify(actual) !== JSON.stringify(identities) ||
    functions.some((fn) => fn.count !== Number(fn.hasExecuted))
  )
    throw new CoverageError('contradictory public native function inventory')
  const merged = framedReloads(capture, reloads)
  return projectNative(
    capture,
    merged.slice(-profiles.length),
    merged.slice(0, -profiles.length),
  )
}

/** Bun reports one row per line for every evaluation of a script, so a block counts when any evaluation ran it. */
function framedReloads(
  capture: ScriptCapture,
  reloads: readonly ScriptCapture[],
): readonly BasicBlock[] {
  const evaluations = reloads.length === 0 ? [capture] : reloads
  for (const item of evaluations) {
    if (
      item.url !== capture.url ||
      item.code !== capture.code ||
      item.sourceMapURL !== capture.sourceMapURL ||
      item.scriptType !== capture.scriptType
    )
      throw new CoverageError('incompatible framed reload source')
  }
  return capture.blocks.map((block) => {
    const hasExecuted = evaluations.some((item) =>
      item.blocks.some(
        (observed) =>
          observed.startOffset === block.startOffset &&
          observed.endOffset === block.endOffset &&
          (observed.hasExecuted || observed.count > 0),
      ),
    )
    return { ...block, hasExecuted, count: Number(hasExecuted) }
  })
}

function reloadUnion(
  capture: ScriptCapture,
  reloads: readonly ScriptCapture[],
): NativeView[] {
  if (reloads.length < 2) return []
  for (const item of reloads) {
    const tail = item.blocks.at(-1)
    if (
      item.profile ||
      item.url !== capture.url ||
      item.code !== capture.code ||
      item.sourceMapURL !== capture.sourceMapURL ||
      item.scriptType !== capture.scriptType ||
      item.blocks.length !== capture.blocks.length ||
      item.blocks.some(
        (block, index) =>
          block.startOffset !== capture.blocks[index]?.startOffset ||
          block.endOffset !== capture.blocks[index]?.endOffset,
      ) ||
      !tail ||
      tail.startOffset !== 0 ||
      tail.endOffset !== capture.code.length - 1 ||
      tail.count !== Number(tail.hasExecuted)
    )
      throw new CoverageError('incompatible unframed reload topology/inventory')
  }
  const blocks = capture.blocks.map((block, index) => {
    const hasExecuted = reloads.some((item) => {
      const observed = item.blocks[index]
      return observed && (observed.hasExecuted || observed.count > 0)
    })
    return { ...block, hasExecuted, count: Number(hasExecuted) }
  })
  return [projectNative(capture, blocks.slice(-1), blocks.slice(0, -1))]
}

function projectNative(
  capture: ScriptCapture,
  functions: readonly BasicBlock[],
  blocks: readonly BasicBlock[],
): NativeView {
  const map = decodeMap(capture.code, capture.sourceMapURL)
  const lines = new Map<number, number>()
  const blockLines = new Set<number>()
  for (const block of blocks) {
    if (block.startOffset < 0 || block.endOffset < 0) continue
    for (const line of mappedLines(map, block)) {
      blockLines.add(line)
      lines.set(
        line,
        (lines.get(line) ?? 0) + Number(block.hasExecuted || block.count > 0),
      )
    }
  }
  const nativeFunctions: NativeFunction[] = []
  const coarse = new Map<number, NativeFunction[]>()
  for (const fn of functions.length > 1 ? functions.slice(1) : functions) {
    if (fn.startOffset < 0 || fn.endOffset < 0) continue
    const mapped = mappedLines(map, fn)
    if (mapped.length === 0) continue
    const identity = {
      startOffset: Math.min(fn.startOffset, fn.endOffset),
      endOffset: Math.max(fn.startOffset, fn.endOffset),
      hasExecuted: fn.hasExecuted,
    }
    nativeFunctions.push(identity)
    if (!fn.hasExecuted) {
      const min = Math.min(...mapped)
      const max = Math.max(...mapped)
      for (let line = min; line < max; line++) {
        lines.set(line, 0)
        coarse.set(line, [...(coarse.get(line) ?? []), identity])
      }
    }
  }
  return {
    code: capture.code,
    map: capture.sourceMapURL,
    input: map.input,
    functions: nativeFunctions,
    lines,
    blockLines,
    coarse,
  }
}

function matchesNative(view: NativeView, source: SourceCoverage): boolean {
  return (
    source.functions.size === 0 &&
    source.branches.size === 0 &&
    source.foundFunctions === view.functions.length &&
    source.hitFunctions ===
      view.functions.filter((fn) => fn.hasExecuted).length &&
    JSON.stringify([...view.lines].sort(([a], [b]) => a - b)) ===
      JSON.stringify([...source.lines].sort(([a], [b]) => a - b))
  )
}

export function validateNative(view: NativeView, source: SourceCoverage): void {
  if (source.functions.size !== 0 || source.branches.size !== 0)
    throw new CoverageError(
      'named/branch data contradicts pinned native producer',
    )
  if (
    source.foundFunctions !== view.functions.length ||
    source.hitFunctions !== view.functions.filter((fn) => fn.hasExecuted).length
  )
    throw new CoverageError(
      `native function lifetime/inventory mismatch: ${source.file}`,
    )
  if (!matchesNative(view, source))
    throw new CoverageError(`native line lifetime/map mismatch: ${source.file}`)
}
