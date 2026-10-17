import {
  isMdxRecord,
  type MdxPipeline,
  requireMdxPipeline,
} from './mdx-pipeline'
import type { MdxDependencyReport } from './mdx-prepare-dependencies'
import type { PreparedMdx } from './mdx-prepare-result'
import type { MdxInputFingerprint } from './mdx-source-freshness'

export type MdxIsolateDemand = {
  readonly run: string
  readonly filename: string
  readonly pipeline: MdxPipeline
  readonly timeoutMs: number
}
export type MdxIsolateFailure = {
  readonly filename: string
  readonly message: string
  readonly line: number
  readonly column: number
}
export type MdxIsolateOutcome =
  | {
      readonly kind: 'prepared'
      readonly output: PreparedMdx
      readonly inputs: readonly MdxInputFingerprint[]
      readonly physical?: readonly MdxIsolatePhysical[]
    }
  | {
      readonly kind: 'failed'
      readonly error: MdxIsolateFailure
      readonly recoverable?: boolean
      readonly terminal?: readonly MdxIsolateVerdict[]
    }

export type MdxIsolatePhysical = {
  readonly loader: string
  readonly module: string
}
export type MdxIsolateVerdict = {
  readonly run: string
  readonly filename: string
  readonly error?: MdxIsolateFailure
  readonly terminal?: readonly MdxIsolateVerdict[]
}
export type MdxIsolateControl = {
  readonly operation: 'check' | 'retire' | 'finalize'
  readonly run: string
  readonly filename: string
  readonly timeoutMs: number
}
export function parseMdxIsolateControl(value: unknown): MdxIsolateControl {
  if (
    !isMdxRecord(value) ||
    (value.operation !== 'check' &&
      value.operation !== 'retire' &&
      value.operation !== 'finalize') ||
    typeof value.run !== 'string' ||
    typeof value.filename !== 'string' ||
    typeof value.timeoutMs !== 'number' ||
    !Number.isFinite(value.timeoutMs) ||
    value.timeoutMs <= 0
  )
    throw new TypeError('invalid MDX isolate control')
  return {
    operation: value.operation,
    run: value.run,
    filename: value.filename,
    timeoutMs: Math.min(value.timeoutMs, 10000),
  }
}
export function parseMdxIsolateVerdict(value: unknown): MdxIsolateVerdict {
  if (
    !isMdxRecord(value) ||
    typeof value.run !== 'string' ||
    typeof value.filename !== 'string'
  )
    throw new TypeError('invalid MDX isolate verdict')
  if (value.terminal !== undefined && !Array.isArray(value.terminal))
    throw new TypeError('invalid MDX isolate terminal verdicts')
  const error =
    value.error === undefined ? undefined : parseMdxIsolateFailure(value.error)
  if (error && error.filename !== value.filename)
    throw new TypeError('MDX isolate verdict error resource mismatch')
  return {
    run: value.run,
    filename: value.filename,
    ...(Array.isArray(value.terminal)
      ? { terminal: value.terminal.map(parseMdxIsolateVerdict) }
      : {}),
    ...(error === undefined ? {} : { error }),
  }
}
function parseMdxIsolateFailure(error: unknown): MdxIsolateFailure {
  if (
    !isMdxRecord(error) ||
    typeof error.filename !== 'string' ||
    typeof error.message !== 'string' ||
    typeof error.line !== 'number' ||
    typeof error.column !== 'number'
  )
    throw new TypeError('invalid MDX isolate failure')
  return {
    filename: error.filename,
    message: error.message,
    line: error.line,
    column: error.column,
  }
}

export class MdxIsolateError extends Error {
  readonly name = 'MdxIsolateError'
  constructor(
    readonly failure: MdxIsolateFailure,
    cleanup?: unknown,
  ) {
    super(failure.message, { cause: cleanup })
  }
}

export function requireMdxIsolateData(value: unknown) {
  const pending = [value]
  const seen = new WeakSet<object>()
  while (pending.length) {
    const input = pending.pop()
    if (
      input === null ||
      input === undefined ||
      typeof input === 'string' ||
      typeof input === 'boolean' ||
      typeof input === 'number'
    )
      continue
    if (input instanceof RegExp) {
      if (
        Object.getPrototypeOf(input) !== RegExp.prototype ||
        Reflect.ownKeys(input).some((key) => key !== 'lastIndex') ||
        input.lastIndex !== 0
      )
        throw new TypeError(
          'MDX isolate cannot clone RegExp subclass/custom state',
        )
      continue
    }
    if (
      typeof input !== 'object' ||
      (!Array.isArray(input) &&
        Object.getPrototypeOf(input) !== Object.prototype)
    )
      throw new TypeError('MDX isolate transport accepts plain DATA only')
    if (seen.has(input)) continue
    seen.add(input)
    for (const key of Reflect.ownKeys(input)) {
      const descriptor = Object.getOwnPropertyDescriptor(input, key)
      if (typeof key !== 'string' || !descriptor || !('value' in descriptor))
        throw new TypeError(
          'MDX isolate transport cannot evaluate accessors or transport symbols',
        )
      pending.push(descriptor.value)
    }
  }
}

export function parseMdxIsolateDemand(value: unknown): MdxIsolateDemand {
  requireMdxIsolateData(value)
  if (
    !isMdxRecord(value) ||
    typeof value.run !== 'string' ||
    typeof value.filename !== 'string' ||
    typeof value.timeoutMs !== 'number' ||
    !Number.isFinite(value.timeoutMs) ||
    value.timeoutMs <= 0 ||
    !isMdxRecord(value.pipeline)
  )
    throw new TypeError('invalid dormant MDX isolate demand')
  const pipeline = value.pipeline
  if (
    pipeline.bundler !== 'turbo' ||
    typeof pipeline.ruleKey !== 'string' ||
    !Array.isArray(pipeline.conditions) ||
    !pipeline.conditions.every(isMdxRecord) ||
    !isMdxRecord(pipeline.aliases) ||
    !Array.isArray(pipeline.loaders) ||
    (pipeline.issue !== undefined && typeof pipeline.issue !== 'string')
  )
    throw new TypeError('dormant MDX isolate requires a Turbo pipeline')
  const loaders = pipeline.loaders.map((loader: unknown) => {
    if (
      !isMdxRecord(loader) ||
      typeof loader.loader !== 'string' ||
      (loader.options !== undefined &&
        typeof loader.options !== 'string' &&
        !isMdxRecord(loader.options))
    )
      throw new TypeError('invalid dormant MDX loader descriptor')
    for (const key of ['ident', 'fragment', 'type'])
      if (loader[key] !== undefined)
        throw new TypeError(
          'Turbo MDX requires the existing wire loader descriptor',
        )
    if (
      Reflect.ownKeys(loader).some(
        (key) => key !== 'loader' && key !== 'options',
      )
    )
      throw new TypeError(
        'MDX isolate cannot discard custom loader descriptor metadata',
      )
    return {
      ...loader,
      loader: loader.loader,
      ...(loader.options === undefined ? {} : { options: loader.options }),
    }
  })
  return {
    run: value.run,
    filename: value.filename,
    timeoutMs: Math.min(value.timeoutMs, 10_000),
    pipeline: requireMdxPipeline(value.filename, {
      ...pipeline,
      bundler: 'turbo',
      ruleKey: pipeline.ruleKey,
      conditions: pipeline.conditions,
      aliases: pipeline.aliases,
      loaders,
      ...(pipeline.issue === undefined ? {} : { issue: pipeline.issue }),
    }),
  }
}

export function parseMdxIsolateOutcome(value: unknown): MdxIsolateOutcome {
  if (!isMdxRecord(value)) throw new TypeError('invalid MDX isolate outcome')
  if (value.kind === 'failed') {
    const error = parseMdxIsolateFailure(value.error)
    if (
      value.recoverable !== undefined &&
      typeof value.recoverable !== 'boolean'
    )
      throw new TypeError('invalid MDX isolate failure scope')
    if (value.terminal !== undefined && !Array.isArray(value.terminal))
      throw new TypeError('invalid MDX isolate terminal batch')
    return {
      kind: 'failed',
      error: {
        filename: error.filename,
        message: error.message,
        line: error.line,
        column: error.column,
      },
      ...(value.recoverable === undefined
        ? {}
        : { recoverable: value.recoverable }),
      ...(Array.isArray(value.terminal)
        ? { terminal: value.terminal.map(parseMdxIsolateVerdict) }
        : {}),
    }
  }
  if (
    value.kind !== 'prepared' ||
    !isMdxRecord(value.output) ||
    !Array.isArray(value.inputs)
  )
    throw new TypeError('invalid MDX isolate prepared result')
  const output = value.output
  const physical = value.physical
  if (
    physical !== undefined &&
    (!Array.isArray(physical) ||
      !physical.every(
        (item: unknown) =>
          isMdxRecord(item) &&
          typeof item.loader === 'string' &&
          typeof item.module === 'string',
      ))
  )
    throw new TypeError('invalid MDX isolate physical sources')
  const strings = (input: unknown): readonly string[] => {
    if (
      !Array.isArray(input) ||
      !input.every((item: unknown): item is string => typeof item === 'string')
    )
      throw new TypeError('invalid MDX isolate dependency list')
    return input
  }
  const report = (input: unknown): MdxDependencyReport => {
    if (
      !isMdxRecord(input) ||
      typeof input.path !== 'string' ||
      typeof input.loader !== 'string' ||
      (input.kind !== 'file' &&
        input.kind !== 'context' &&
        input.kind !== 'missing' &&
        input.kind !== 'build')
    )
      throw new TypeError('invalid MDX isolate dependency report')
    return { kind: input.kind, path: input.path, loader: input.loader }
  }
  if (
    typeof output.filename !== 'string' ||
    typeof output.source !== 'string' ||
    (output.map !== undefined &&
      typeof output.map !== 'string' &&
      !isMdxRecord(output.map)) ||
    !Array.isArray(output.dependencyReports)
  )
    throw new TypeError('invalid MDX isolate compiled output')
  return {
    kind: 'prepared',
    ...(Array.isArray(physical)
      ? {
          physical: physical.map((item: unknown) => {
            if (
              !isMdxRecord(item) ||
              typeof item.loader !== 'string' ||
              typeof item.module !== 'string'
            )
              throw new TypeError('invalid MDX isolate physical source')
            return { loader: item.loader, module: item.module }
          }),
        }
      : {}),
    output: {
      filename: output.filename,
      source: output.source,
      map: output.map,
      dependencies: strings(output.dependencies),
      contextDependencies: strings(output.contextDependencies),
      missingDependencies: strings(output.missingDependencies),
      buildDependencies: strings(output.buildDependencies),
      dependencyReports: output.dependencyReports.map(report),
    },
    inputs: value.inputs.map((input: unknown) => {
      const dependency = report(input)
      if (
        !isMdxRecord(input) ||
        typeof input.fingerprint !== 'string' ||
        typeof input.mtime !== 'number'
      )
        throw new TypeError('invalid MDX isolate input fingerprint')
      return {
        ...dependency,
        fingerprint: input.fingerprint,
        mtime: input.mtime,
      }
    }),
  }
}
