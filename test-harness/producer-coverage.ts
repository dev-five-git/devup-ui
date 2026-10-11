import { relative, resolve } from 'node:path'

import { CoverageError, parseLcov, type SourceCoverage } from './lcov'
import { array, integer, object, text } from './producer-data'
import {
  type NativeView,
  reconstructNative,
  validateNative,
} from './producer-native'
import { isProducerResult, type ProducerResult } from './producer-process'

const issuance = Symbol()
const issued = new WeakSet<object>()

export function isVerifiedSourceCoverage(
  value: object,
): value is VerifiedSourceCoverage {
  return issued.has(value)
}

export class VerifiedSourceCoverage implements SourceCoverage {
  readonly file: string
  readonly foundFunctions: number
  readonly hitFunctions: number
  readonly #view: NativeView
  readonly #configuration: string

  private constructor(
    capability: typeof issuance,
    payload: {
      readonly source: SourceCoverage
      readonly view: NativeView
      readonly configuration: string
    },
  ) {
    if (capability !== issuance)
      throw new CoverageError('unproven native coverage issuance')
    const { source, view, configuration } = payload
    this.file = source.file
    this.#view = { ...view, lines: new Map(source.lines) }
    this.#configuration = configuration
    this.foundFunctions = view.functions.length
    this.hitFunctions = view.functions.filter((fn) => fn.hasExecuted).length
    Object.freeze(this)
    issued.add(this)
  }

  get lines(): Map<number, number> {
    return new Map(this.#view.lines)
  }

  get functions(): Map<string, { readonly line: number; hits: number }> {
    return new Map()
  }

  get branches(): Map<string, number> {
    return new Map()
  }

  static bind(result: ProducerResult, root: string): VerifiedSourceCoverage[] {
    if (!isProducerResult(result))
      throw new CoverageError('unproven native producer lifetime')
    const sources = parseLcov(result.value.lcov)
    const config = object(object(Bun.TOML.parse(result.value.config))['test'])
    const ignored = array(config['coveragePathIgnorePatterns'] ?? []).map(
      (value) => new Bun.Glob(text(value)),
    )
    for (const capture of result.value.captures) {
      const path = relative(root, capture.url).replaceAll('\\', '/')
      if (
        capture.blocks.length > 0 &&
        !ignored.some((pattern) => pattern.match(path)) &&
        !sources.some((source) => resolve(root, source.file) === capture.url)
      )
        throw new CoverageError(
          `missing native source from actual producer inventory: ${path}`,
        )
    }
    return sources.map((source) => {
      const path = resolve(root, source.file).replaceAll('\\', '/')
      const captures = result.value.captures.filter(
        (capture) => capture.url.replaceAll('\\', '/') === path,
      )
      const capture = [...captures]
        .sort(
          (a, b) => integer(Number(a.scriptId)) - integer(Number(b.scriptId)),
        )
        .at(-1)
      if (!capture) throw new CoverageError('missing producer capture')
      if (
        captures.some(
          (item) =>
            item.code !== capture.code ||
            item.sourceMapURL !== capture.sourceMapURL ||
            item.scriptType !== capture.scriptType,
        )
      )
        throw new CoverageError(
          `incompatible reloaded producer source: ${source.file}`,
        )
      // Bun 1.4.2 registers coverage at SourceProvider creation; pinned WebKit assigns monotonically increasing SourceIDs.
      const view = reconstructNative(capture, source)
      validateNative(view, source)
      return new VerifiedSourceCoverage(issuance, {
        source,
        view,
        configuration: result.value.config,
      })
    })
  }

  merge(other: VerifiedSourceCoverage): VerifiedSourceCoverage {
    if (!isVerifiedSourceCoverage(this) || !isVerifiedSourceCoverage(other))
      throw new CoverageError('unproven native coverage merge')
    if (
      this.#view.code !== other.#view.code ||
      this.#view.map !== other.#view.map ||
      this.#configuration !== other.#configuration
    )
      throw new CoverageError(
        `incompatible effective producer code/map/config: ${this.file}`,
      )
    const identities = new Map(
      this.#view.functions.map((fn) => [
        `${fn.startOffset},${fn.endOffset}`,
        fn,
      ]),
    )
    for (const fn of other.#view.functions) {
      const key = `${fn.startOffset},${fn.endOffset}`
      const previous = identities.get(key)
      identities.set(key, {
        ...fn,
        hasExecuted: fn.hasExecuted || previous?.hasExecuted === true,
      })
    }
    const lines = new Map(this.lines)
    for (const [line, hits] of other.lines)
      lines.set(line, (lines.get(line) ?? 0) + hits)
    for (const [line, hits] of lines) {
      const left = this.#view.coarse.get(line) ?? []
      const right = other.#view.coarse.get(line) ?? []
      const executedBy = (
        origins: readonly {
          readonly startOffset: number
          readonly endOffset: number
        }[],
        peer: NativeView,
      ) =>
        origins.length > 0 &&
        origins.every((origin) =>
          peer.functions.some(
            (fn) =>
              fn.startOffset === origin.startOffset &&
              fn.endOffset === origin.endOffset &&
              fn.hasExecuted,
          ),
        )
      if (
        hits === 0 &&
        !this.#view.blockLines.has(line) &&
        !other.#view.blockLines.has(line) &&
        ((executedBy(left, other.#view) && !other.lines.has(line)) ||
          (executedBy(right, this.#view) && !this.lines.has(line)))
      )
        lines.delete(line)
    }
    const coarse = new Map(this.#view.coarse)
    for (const [line, origins] of other.#view.coarse)
      coarse.set(line, [...(coarse.get(line) ?? []), ...origins])
    const view = {
      ...this.#view,
      functions: [...identities.values()],
      lines,
      blockLines: new Set([
        ...this.#view.blockLines,
        ...other.#view.blockLines,
      ]),
      coarse,
    }
    return new VerifiedSourceCoverage(issuance, {
      source: {
        file: this.file,
        lines,
        functions: this.functions,
        branches: this.branches,
        foundFunctions: this.foundFunctions,
        hitFunctions: this.hitFunctions,
      },
      view,
      configuration: this.#configuration,
    })
  }
}
