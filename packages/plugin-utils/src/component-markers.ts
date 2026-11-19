/**
 * Component-selector markers a build can drop.
 *
 * A styled component that a module exports carries a marker class, because a
 * module importing it may select it (`${Child} { ... }`, `[Child]`). Only a
 * build that has seen every module can know that none does. The start-of-build
 * scan extracts the inventory and reports the markers its styles select; each
 * transform reports the markers its module gives and selects. A one-shot
 * production build drops a marker from its output chunks only when:
 *
 * - the scan ran, and every transformed module selects only markers the scan
 *   saw, so every compilation of the build, in this process or another one,
 *   reads the same list;
 * - no module selects it, in the scan or in a transform; and
 * - no module wrote the class itself, so removing it touches only Devup UI's
 *   own class lists.
 *
 * Otherwise it keeps every marker and says why, naming each module the scan
 * did not cover and the reason, so the cases can be closed one by one.
 */

/** Why the start-of-build scan did not see a module's selectors */
export type UnscannedCause =
  /** A module no file backs, made by a plugin (`virtual:`, `\0` ids) */
  | 'virtual-module'
  /** A file another tool compiles first (MDX, Vue, Svelte, Astro) */
  | 'compiled-source'
  /** Loaded through `require.context`, `import.meta.glob` or a computed `import()` the scan cannot follow */
  | 'context-module'
  /** A source file the scan never reached from its roots and entries */
  | 'outside-scan'
  /** A file the scan read but could not extract */
  | 'scan-failed'

/** A module whose selectors the scan did not see */
export interface UnscannedModule {
  readonly id: string
  readonly cause: UnscannedCause
  /** The markers it selects that the scan did not report */
  readonly markers: readonly string[]
}

/** Why a build keeps every component-selector marker */
export type MarkerKeepReason =
  /** Dev servers and watch builds: modules change after their markers are given */
  | { readonly kind: 'watch' }
  /** No start-of-build scan reported its selections */
  | { readonly kind: 'no-scan' }
  /** Modules selected markers the scan did not see */
  | {
      readonly kind: 'unscanned'
      readonly modules: readonly UnscannedModule[]
    }
  /** An earlier compilation in this process found modules the scan missed */
  | { readonly kind: 'earlier-compilation' }

/** What a build does with the markers its modules give */
export type MarkerPlan =
  { readonly strip: ReadonlySet<string> } | { readonly keep: MarkerKeepReason }

/** What one transform reports */
export interface MarkerTransform {
  /** The id the bundler knows the module by */
  readonly id: string
  /** The code the transform read */
  readonly input: string
  readonly givenMarkers: readonly string[]
  readonly selectedMarkers: readonly string[]
  /** Why the scan may not have seen this module, as the bundler knows it */
  readonly cause?: UnscannedCause
}

interface Recorded {
  readonly given: readonly string[]
  readonly selected: readonly string[]
  /** Class-like tokens the module wrote itself */
  readonly authored: ReadonlySet<string>
  readonly cause: UnscannedCause
}

/** The classes a build may never touch, across the compilations of a process */
const processState: {
  uncertain: boolean
  stripped: Set<string>
} = { uncertain: false, stripped: new Set() }

/** @internal Forget what earlier compilations of this process found */
export function resetMarkerProcessState(): void {
  processState.uncertain = false
  processState.stripped = new Set()
}

const TOKEN = /[^\s"'`${}]+--[^\s"'`${}]*/g

/** The tokens of `code` written like a marker class (`a--b`) */
export function markerLikeTokens(code: string): Set<string> {
  return new Set(code.match(TOKEN))
}

const SOURCE_SCRIPT = /\.[cm]?[jt]sx?$/i

/** Why the scan may have missed module `id`, from what the bundler read */
export function unscannedCause(
  id: string,
  input: string,
  onDisk: string | undefined,
): UnscannedCause {
  if (id.includes('\0') || id.startsWith('virtual:') || onDisk === undefined)
    return 'virtual-module'
  if (!SOURCE_SCRIPT.test(id.split('?')[0] ?? id) || onDisk !== input)
    return 'compiled-source'
  return 'outside-scan'
}

/** Gathers what the scan and the transforms of one build report */
export class ComponentMarkerLedger {
  private scan: ReadonlySet<string> | undefined
  private scannedIds: ReadonlySet<string> = new Set()
  private readonly modules = new Map<string, Recorded>()

  /**
   * Take the markers the scan's styles select, and the ids of the modules it
   * extracted; `undefined` when no scan reported them.
   */
  scanned(
    selections: readonly string[] | void,
    modules: Iterable<string>,
  ): void {
    this.scan = selections ? new Set(selections) : undefined
    this.scannedIds = new Set(modules)
  }

  /** Take what one transform reports, replacing an earlier one of the module */
  record(transform: MarkerTransform): void {
    this.modules.set(transform.id, {
      given: transform.givenMarkers,
      selected: transform.selectedMarkers,
      authored: markerLikeTokens(transform.input),
      cause:
        transform.cause ??
        (this.scannedIds.has(transform.id) ? 'scan-failed' : 'outside-scan'),
    })
  }

  /** Whether a transform of module `id` reported to this ledger */
  recorded(id: string): boolean {
    return this.modules.has(id)
  }

  /** Forget every transform, as a new build begins */
  clear(): void {
    this.modules.clear()
  }

  /** What the build does with its markers, once every module is transformed */
  plan(): MarkerPlan {
    if (processState.uncertain) return { keep: { kind: 'earlier-compilation' } }
    const scan = this.scan
    if (!scan) return { keep: { kind: 'no-scan' } }
    const unscanned = this.unscanned(scan)
    if (unscanned.length) {
      processState.uncertain = true
      return { keep: { kind: 'unscanned', modules: unscanned } }
    }
    const strip = this.unselected(scan)
    for (const marker of strip) processState.stripped.add(marker)
    return { strip }
  }

  /** Markers an earlier compilation of this process already dropped */
  static strippedEarlier(markers: readonly string[]): string[] {
    return markers.filter((marker) => processState.stripped.has(marker))
  }

  private unscanned(scan: ReadonlySet<string>): UnscannedModule[] {
    const modules: UnscannedModule[] = []
    for (const [id, recorded] of this.modules) {
      const markers = recorded.selected.filter((marker) => !scan.has(marker))
      if (markers.length) modules.push({ id, cause: recorded.cause, markers })
    }
    return modules.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
  }

  private unselected(scan: ReadonlySet<string>): Set<string> {
    const kept = new Set(scan)
    for (const recorded of this.modules.values()) {
      for (const marker of recorded.selected) kept.add(marker)
      for (const token of recorded.authored) kept.add(token)
    }
    const strip = new Set<string>()
    for (const recorded of this.modules.values())
      for (const marker of recorded.given)
        if (!kept.has(marker)) strip.add(marker)
    return strip
  }
}

const CAUSES: Record<UnscannedCause, string> = {
  'virtual-module': 'a virtual module no file backs',
  'compiled-source': 'compiled by another tool before Devup UI read it',
  'context-module':
    'loaded through require.context, import.meta.glob or a computed import()',
  'outside-scan': 'not reachable from the scanned roots and entries',
  'scan-failed': 'the start-of-build scan could not extract it',
}

/**
 * The warning for a build that keeps its markers because certainty is
 * missing; `undefined` when there is nothing to say.
 */
export function markerWarning(reason: MarkerKeepReason): string | undefined {
  if (reason.kind !== 'unscanned') return undefined
  const lines = reason.modules.map(
    (module) =>
      `  - ${module.id} [${module.cause}]: ${CAUSES[module.cause]}; selects ${module.markers.join(', ')}`,
  )
  const earlier = ComponentMarkerLedger.strippedEarlier(
    reason.modules.flatMap((module) => module.markers),
  )
  if (earlier.length)
    lines.push(
      `  An earlier compilation of this build already dropped ${earlier.join(', ')}; rebuild to keep them there too.`,
    )
  return [
    '[devup-ui] Kept every component-selector marker in this build: the start-of-build scan did not see these modules select components from other modules.',
    ...lines,
  ].join('\n')
}

/** A removal from a chunk, as `[start, end)` offsets into its code */
export type MarkerEdit = readonly [start: number, end: number]

const QUOTE = new Set(['"', "'", '`'])

function isClassBoundary(char: string | undefined): boolean {
  return char === undefined || char === ' ' || QUOTE.has(char)
}

/**
 * The removals that drop `markers` from the class lists in `code`: each one
 * standing alone between quotes or spaces, with the space that separates it
 */
export function markerEdits(
  code: string,
  markers: ReadonlySet<string>,
): MarkerEdit[] {
  const edits: MarkerEdit[] = []
  if (!markers.size) return edits
  for (const match of code.matchAll(TOKEN)) {
    const start = match.index
    const end = start + match[0].length
    if (
      !markers.has(match[0]) ||
      !isClassBoundary(code[start - 1]) ||
      !(isClassBoundary(code[end]) || code.startsWith('${', end))
    )
      continue
    if (code[end] === ' ') edits.push([start, end + 1])
    else if (code[start - 1] === ' ') edits.push([start - 1, end])
    else edits.push([start, end])
  }
  return edits
}

/** `code` with `edits` removed */
export function applyMarkerEdits(
  code: string,
  edits: readonly MarkerEdit[],
): string {
  let result = ''
  let at = 0
  for (const [start, end] of edits) {
    result += code.slice(at, start)
    at = end
  }
  return result + code.slice(at)
}

const BASE64 =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'

function vlq(value: number): string {
  let rest = value < 0 ? (-value << 1) | 1 : value << 1
  let encoded = ''
  do {
    let digit = rest & 31
    rest >>>= 5
    if (rest > 0) digit |= 32
    encoded += BASE64[digit]
  } while (rest > 0)
  return encoded
}

const WORD = /[\w$]/

/**
 * A source map from `code` with `edits` removed back to `code`, with a
 * segment at every word boundary so that the map it is composed with keeps
 * its precision
 */
export function markerEditMap(
  code: string,
  edits: readonly MarkerEdit[],
): { version: 3; sources: string[]; names: string[]; mappings: string } {
  const lines: string[] = []
  let segments: string[] = []
  let generated = 0
  let previousGenerated = 0
  let previousLine = 0
  let previousColumn = 0
  let line = 0
  let column = 0
  let edit = 0
  const segment = () => {
    segments.push(
      vlq(generated - previousGenerated) +
        'A' +
        vlq(line - previousLine) +
        vlq(column - previousColumn),
    )
    previousGenerated = generated
    previousLine = line
    previousColumn = column
  }
  for (let index = 0; index < code.length; index++) {
    const current = edits[edit]
    if (current && index === current[0]) {
      column += current[1] - current[0]
      index = current[1] - 1
      edit++
      continue
    }
    const char = code[index]
    if (
      column === 0 ||
      WORD.test(char ?? '') !== WORD.test(code[index - 1] ?? '')
    )
      segment()
    if (char === '\n') {
      lines.push(segments.join(','))
      segments = []
      generated = 0
      previousGenerated = 0
      line++
      column = 0
    } else {
      generated++
      column++
    }
  }
  lines.push(segments.join(','))
  return { version: 3, sources: [''], names: [], mappings: lines.join(';') }
}

const LEDGERS = Symbol.for('devup-ui.componentMarkerLedgers')

function ledgers(): WeakMap<object, ComponentMarkerLedger> {
  const scope = globalThis as typeof globalThis & {
    [LEDGERS]?: WeakMap<object, ComponentMarkerLedger>
  }
  return (scope[LEDGERS] ??= new WeakMap())
}

/**
 * The ledger of the build `owner` stands for, as the plugin registered it;
 * shared by every copy of this module a process loads, so that a loader
 * bundled apart from its plugin reports to the same ledger
 */
export function markerLedgerOf(
  owner: object | undefined,
): ComponentMarkerLedger | undefined {
  return owner ? ledgers().get(owner) : undefined
}

/** Register `ledger` as the one of the build `owner` stands for */
export function registerMarkerLedger(
  owner: object,
  ledger: ComponentMarkerLedger,
): void {
  ledgers().set(owner, ledger)
}
