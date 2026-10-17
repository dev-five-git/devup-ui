import { existsSync } from 'node:fs'
import { dirname, isAbsolute, resolve } from 'node:path'

import { isRecord, parseJsonc } from './jsonc'
import { ConfigLoadError } from './load-config'
import { resolvePackage } from './owned-module-resolution'
import { isResolutionFile } from './packaged-config-inputs'
import {
  readResolutionFile,
  realpathResolution,
  recordResolutionFailure,
  type ResolutionInputCollector,
} from './resolution-inputs'

export interface PathAlias {
  readonly prefix: string
  readonly suffix: string
  readonly exact: boolean
  readonly targets: string[]
}

interface AliasConfig {
  readonly baseUrl?: string
  readonly paths?: Record<string, string[]>
  readonly pathsOrigin?: string
}

function resolveParent(
  specifier: string,
  file: string,
  inputs?: ResolutionInputCollector,
): string {
  const request = specifier.replaceAll('\\', '/')
  if (
    request.startsWith('./') ||
    request.startsWith('../') ||
    isAbsolute(request)
  ) {
    const candidate = resolve(dirname(file), request)
    return isResolutionFile(candidate, inputs) || candidate.endsWith('.json')
      ? candidate
      : `${candidate}.json`
  }
  const selected = resolvePackage(request, file, {
    purpose: 'tsconfig-extends',
    inputs,
  })
  if (typeof selected === 'string') return selected
  throw new ConfigLoadError(
    file,
    new Error(`Cannot resolve tsconfig extends ${JSON.stringify(specifier)}`),
  )
}

function loadAliases(
  file: string,
  stack: readonly string[],
  inputs?: ResolutionInputCollector,
): AliasConfig {
  try {
    const canonical = realpathResolution(file, inputs)
    if (stack.includes(canonical) || stack.length >= 128)
      throw new Error(
        `Tsconfig inheritance cycle or depth limit: ${[...stack, canonical].join(' -> ')}`,
      )
    const config = parseJsonc(readResolutionFile(file, inputs))
    if (!isRecord(config)) throw new TypeError('Expected a tsconfig object')
    const parents =
      config.extends === undefined
        ? []
        : typeof config.extends === 'string'
          ? [config.extends]
          : config.extends
    if (
      !Array.isArray(parents) ||
      !parents.every((parent) => typeof parent === 'string')
    )
      throw new TypeError('Expected extends to be a path or array of paths')
    let inherited: AliasConfig = {}
    for (const parent of parents)
      inherited = {
        ...inherited,
        ...loadAliases(
          resolveParent(parent, file, inputs),
          [...stack, canonical],
          inputs,
        ),
      }
    if (config.compilerOptions === undefined) return inherited
    if (!isRecord(config.compilerOptions))
      throw new TypeError('Expected compilerOptions to be an object')
    const options = config.compilerOptions
    const current: {
      baseUrl?: string
      paths?: Record<string, string[]>
      pathsOrigin?: string
    } = {}
    if (options.baseUrl !== undefined) {
      if (typeof options.baseUrl !== 'string')
        throw new TypeError('Expected baseUrl to be a path')
      current.baseUrl = resolve(dirname(file), options.baseUrl)
    }
    if (options.paths !== undefined) {
      if (!isRecord(options.paths))
        throw new TypeError('Expected paths to be an object')
      current.paths = {}
      current.pathsOrigin = dirname(file)
      for (const [key, targets] of Object.entries(options.paths)) {
        if (
          !Array.isArray(targets) ||
          !targets.every(
            (target): target is string => typeof target === 'string',
          )
        )
          throw new TypeError(`Expected string targets for paths.${key}`)
        current.paths[key] = targets
      }
    }
    return { ...inherited, ...current }
  } catch (cause) {
    if (cause instanceof ConfigLoadError) throw cause
    recordResolutionFailure(file, cause, inputs)
    throw new ConfigLoadError(file, cause)
  }
}

export function readPathAliases(
  tsconfigPath?: string,
  inputs?: ResolutionInputCollector,
): {
  aliases: PathAlias[]
  baseDir: string
  baseUrl?: string
} {
  const exists = tsconfigPath ? existsSync(tsconfigPath) : false
  if (tsconfigPath) inputs?.probe(tsconfigPath, exists)
  if (!tsconfigPath || !exists) return { aliases: [], baseDir: process.cwd() }
  const file = resolve(tsconfigPath)
  const config = loadAliases(file, [], inputs)
  const baseDir = config.baseUrl ?? config.pathsOrigin ?? dirname(file)
  const aliases = Object.entries(config.paths ?? {}).map(([alias, targets]) => {
    const star = alias.indexOf('*')
    return {
      prefix: star === -1 ? alias : alias.slice(0, star),
      suffix: star === -1 ? '' : alias.slice(star + 1),
      exact: star === -1,
      targets: targets.map((target) => resolve(baseDir, target)),
    }
  })
  aliases.sort(
    (a, b) =>
      Number(b.exact) - Number(a.exact) || b.prefix.length - a.prefix.length,
  )
  return {
    aliases,
    baseDir: config.baseUrl ?? dirname(file),
    baseUrl: config.baseUrl,
  }
}
