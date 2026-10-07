import { dirname, isAbsolute, join, resolve } from 'node:path'

import { isRecord } from './jsonc'
import { ConfigLoadError } from './load-config'
import { resolveJsonFile } from './owned-file-resolution'
import { type ConfigPolicy, resolvePackage } from './owned-module-resolution'
import { conditionTarget, matchConfigTarget } from './package-target'
import {
  isResolutionFile as isFile,
  readPackageManifest,
} from './packaged-config-inputs'

interface PackageScope extends ConfigPolicy {
  readonly directory: string
  readonly imports?: boolean
}

export function configExport(
  table: unknown,
  request: string,
  scope: PackageScope,
): string | null | undefined {
  let match: ReturnType<typeof matchConfigTarget>
  if (scope.imports) match = matchConfigTarget(table, request)
  else if (request === '.') {
    const target =
      typeof table === 'string' ||
      Array.isArray(table) ||
      (isRecord(table) &&
        !Object.keys(table).some((key) => key.startsWith('.')))
        ? table
        : isRecord(table)
          ? table['.']
          : undefined
    if (target) match = { target, subpath: '', pattern: false }
  } else if (
    isRecord(table) &&
    Object.keys(table).every((key) => key.startsWith('.'))
  )
    match = matchConfigTarget(table, request)
  if (!match) return undefined
  const { target, subpath, pattern } = match
  return conditionTarget(target, {
    purpose: 'tsconfig-extends',
    conditions: ['require', 'types', 'node'],
    load(value) {
      if (!pattern && subpath && !value.endsWith('/')) return undefined
      if (!value.startsWith('./')) {
        if (scope.imports && !value.startsWith('../') && !isAbsolute(value)) {
          const request = pattern
            ? value.replaceAll('*', subpath)
            : value + subpath
          const stack = scope.stack ?? []
          if (stack.includes(request))
            throw new ConfigLoadError(
              join(scope.directory, 'package.json'),
              new Error(
                `Package imports cycle: ${[...stack, request].join(' -> ')}`,
              ),
            )
          const found = resolvePackage(
            request,
            join(scope.directory, 'tsconfig.json'),
            { ...scope, configLookup: false, stack: [...stack, request] },
          )
          return typeof found === 'string' ? found : undefined
        }
        return undefined
      }
      const prohibited = (parts: readonly string[]) =>
        parts.some(
          (part) => part === '.' || part === '..' || part === 'node_modules',
        )
      const targetPath = value.replaceAll('\\', '/')
      const targetSubpath = subpath.replaceAll('\\', '/')
      if (
        prohibited(targetPath.split('/').slice(1)) ||
        prohibited(targetSubpath.split('/'))
      )
        return undefined
      const selected = resolve(
        scope.directory,
        pattern
          ? targetPath.replaceAll('*', targetSubpath)
          : targetPath + targetSubpath,
      )
      return resolveJsonFile(selected, { ...scope, exact: true })
    },
  })
}

export function configScopeTarget(
  request: string,
  importer: string,
  options: ConfigPolicy,
): string | null | undefined {
  if (request === '#') return null
  const directory = dirname(importer)
  const manifestPath = join(directory, 'package.json')
  if (isFile(manifestPath, options.inputs)) {
    const manifest = readPackageManifest(
      manifestPath,
      'tsconfig-extends',
      options.inputs,
    )
    if (request.startsWith('#'))
      return (
        configExport(manifest.imports, request, {
          ...options,
          directory,
          imports: true,
        }) ?? null
      )
    const name = manifest.name
    if (
      manifest.exports &&
      typeof name === 'string' &&
      (request === name || request.startsWith(`${name}/`))
    )
      return configExport(
        manifest.exports,
        request === name ? '.' : `.${request.slice(name.length)}`,
        { ...options, directory },
      )
    return undefined
  }
  const parent = dirname(directory)
  if (parent === directory) return request.startsWith('#') ? null : undefined
  return configScopeTarget(request, join(parent, 'tsconfig.json'), options)
}
