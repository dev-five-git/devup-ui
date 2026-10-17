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
): NativeView {
  if (!capture.profile) {
    if (!source)
      throw new CoverageError(
        'missing native observation for unframed public blocks',
      )
    const candidates: NativeView[] = []
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
  const blocks = capture.blocks.slice(0, -profiles.length)
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
  return projectNative(capture, functions, blocks)
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
