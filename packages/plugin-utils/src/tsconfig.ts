import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, isAbsolute, join, resolve } from 'node:path'

import { ConfigLoadError } from './load-config'
import { recordPackagedConfigInputs } from './packaged-config-inputs'
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function parseJsonc(source: string): unknown {
  let text = ''
  let quoted = false
  for (let index = 0; index < source.length; index += 1) {
    const char = source[index]
    const next = source[index + 1]
    if (quoted) {
      text += char
      if (char === '\\') text += source[++index] ?? ''
      else if (char === '"') quoted = false
    } else if (char === '"') {
      quoted = true
      text += char
    } else if (char === '/' && next === '/') {
      while (index < source.length && source[index] !== '\n') index += 1
      text += '\n'
    } else if (char === '/' && next === '*') {
      const end = source.indexOf('*/', index + 2)
      if (end === -1) throw new SyntaxError('Unterminated JSON comment')
      text += ' '
      index = end + 1
    } else text += char
  }
  let json = ''
  quoted = false
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index]
    if (quoted) {
      json += char
      if (char === '\\') json += text[++index] ?? ''
      else if (char === '"') quoted = false
    } else if (char === '"') {
      quoted = true
      json += char
    } else if (char !== ',' || !/^\s*[}\]]/.test(text.slice(index + 1)))
      json += char
  }
  return JSON.parse(json)
}

function resolveParent(
  specifier: string,
  file: string,
  inputs?: ResolutionInputCollector,
): string {
  if (specifier.startsWith('.') || isAbsolute(specifier)) {
    const candidate = resolve(dirname(file), specifier)
    const exists = existsSync(candidate)
    inputs?.probe(candidate, exists)
    return exists ? candidate : `${candidate}.json`
  }
  const require = createRequire(file)
  let selected: string | undefined
  try {
    try {
      const resolved = require.resolve(specifier)
      selected = resolved
      if (resolved.endsWith('.json')) return resolved
      const manifestPath = require.resolve(`${specifier}/package.json`)
      const manifest: unknown = JSON.parse(
        readResolutionFile(manifestPath, inputs),
      )
      return join(
        dirname(manifestPath),
        isRecord(manifest) && typeof manifest.tsconfig === 'string'
          ? manifest.tsconfig
          : 'tsconfig.json',
      )
    } catch (cause) {
      if (
        !(cause instanceof Error) ||
        !('code' in cause) ||
        cause.code !== 'MODULE_NOT_FOUND'
      )
        throw cause
      try {
        const resolved = require.resolve(`${specifier}/tsconfig.json`)
        selected = resolved
        return resolved
      } catch (fallback) {
        if (
          !(fallback instanceof Error) ||
          !('code' in fallback) ||
          fallback.code !== 'MODULE_NOT_FOUND'
        )
          throw fallback
        const manifestPath = require.resolve(`${specifier}/package.json`)
        selected = manifestPath
        const manifest: unknown = JSON.parse(
          readResolutionFile(manifestPath, inputs),
        )
        const configName =
          isRecord(manifest) && typeof manifest.tsconfig === 'string'
            ? manifest.tsconfig
            : 'tsconfig.json'
        return join(dirname(manifestPath), configName)
      }
    }
  } finally {
    recordPackagedConfigInputs(
      { request: specifier, importer: file, resolved: selected },
      inputs,
    )
  }
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
