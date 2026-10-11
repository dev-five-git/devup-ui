import { extname } from 'node:path'

import type {
  ModuleResolution,
  PreparedSource,
  PrepareSource,
  ResolvedModule,
} from './import-graph'
import { remapMdxError } from './mdx-errors'
import { readPreparedSource } from './prepared-source'
import {
  createResolutionInputs,
  readResolutionFile,
  type ResolutionInputObserver,
} from './resolution-inputs'
import type { MdxSelection } from './source-selection'

export interface ModuleResolver {
  (specifier: string, importer: string): ModuleResolution | undefined
  remapError(error: unknown): Error
}

class ModulePreparationError extends Error {
  constructor(importer: string, filename: string, cause: unknown) {
    super(
      `${importer}:1:1: module source preparation cannot use \`${filename}\` at build time: ${cause instanceof Error ? cause.message : String(cause)}; prepare Markdown before extraction using a synchronous cache lookup, or move the imported value into JS/TS`,
      { cause },
    )
    this.name = 'ModulePreparationError'
  }
}

function isThenable(value: unknown): value is PromiseLike<unknown> {
  return (
    value !== null &&
    (typeof value === 'object' || typeof value === 'function') &&
    'then' in value &&
    typeof value.then === 'function'
  )
}

/** The mutable store belongs to one resolver, not to the WASM singleton. */
export function createPreparedResolver(options: {
  readonly prepareSource?: PrepareSource
  readonly includeMdx?: MdxSelection
  readonly onResolutionInputs?: ResolutionInputObserver
}) {
  const generations = new Map<
    string,
    {
      readonly filename: string
      readonly code: string
      readonly map?: unknown
      readonly sourceType?: import('./prepared-source').SourceType
    }
  >()
  const idsByFilename = new Map<string, string>()
  const remappedErrors = new WeakSet<Error>()
  const markdownExtensions = new Set([
    '.md',
    '.mdx',
    ...(typeof options.includeMdx === 'object'
      ? options.includeMdx.map((extension) => extension.toLowerCase())
      : []),
  ])
  return {
    read(filename: string, importer: string): PreparedSource {
      const previousId = idsByFilename.get(filename)
      if (previousId !== undefined) generations.delete(previousId)
      idsByFilename.delete(filename)
      try {
        const prepared = options.prepareSource?.(filename)
        if (isThenable(prepared))
          throw new TypeError(
            'Source is not ready: the synchronous WASM resolver received a thenable',
          )
        if (
          prepared === undefined &&
          markdownExtensions.has(extname(filename).toLowerCase())
        )
          throw new TypeError('Markdown source has no prepared JavaScript')
        const source = readPreparedSource(prepared)
        if (source !== undefined) {
          const inputs = createResolutionInputs()
          inputs.file(filename)
          options.onResolutionInputs?.(inputs.snapshot())
        }
        return source
      } catch (cause) {
        throw new ModulePreparationError(importer, filename, cause)
      }
    },
    resolved(
      id: string,
      filename: string,
      prepared: PreparedSource,
    ): ResolvedModule {
      if (prepared === undefined) {
        generations.delete(id)
        const inputs = createResolutionInputs()
        try {
          return { path: id, code: readResolutionFile(filename, inputs) }
        } finally {
          options.onResolutionInputs?.(inputs.snapshot())
        }
      }
      const source =
        typeof prepared === 'string' ? { code: prepared } : prepared
      generations.set(id, { filename, ...source })
      idsByFilename.set(filename, id)
      return {
        path: id,
        code: source.code,
        ...(source.sourceType === undefined
          ? {}
          : { sourceType: source.sourceType }),
      }
    },
    remapError(error: unknown): Error {
      if (error instanceof Error && remappedErrors.has(error)) return error
      const message = error instanceof Error ? error.message : String(error)
      const replacements = [...generations]
        .flatMap(([id, source]) => {
          const escaped = id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
          const pattern = new RegExp(
            `(^|[\\s"'\x60])${escaped}:(\\d+):(\\d+)(?=:|\\s|$)`,
            'g',
          )
          return [...message.matchAll(pattern)].map((match) => ({
            start: match.index + match[1].length,
            end: match.index + match[0].length,
            text: remapMdxError(
              `${source.filename}:${match[2]}:${match[3]}`,
              source.filename,
              source.map,
            ).message.replace('(in compiled MDX)', '(in compiled output)'),
          }))
        })
        .sort((a, b) => a.start - b.start)
      const mapped = replacements.reduce(
        (state, replacement) => ({
          text:
            state.text +
            message.slice(state.end, replacement.start) +
            replacement.text,
          end: replacement.end,
        }),
        { text: '', end: 0 },
      )
      const result = new Error(mapped.text + message.slice(mapped.end), {
        cause: error,
      })
      remappedErrors.add(result)
      return result
    },
  }
}
