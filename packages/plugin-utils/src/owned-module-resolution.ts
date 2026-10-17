import { basename, dirname, join, resolve } from 'node:path'

import { ModuleAliasPackageError } from './module-alias'
import { exportsTarget } from './package-target'
import {
  isResolutionFile as isFile,
  readPackageManifest,
} from './packaged-config-inputs'
import {
  realpathResolution,
  type ResolutionInputCollector,
} from './resolution-inputs'
import { configScopeTarget } from './tsconfig-export-policy'
import {
  configPackageCandidate,
  directoryConfig,
} from './tsconfig-package-policy'

interface ModulePolicy {
  readonly purpose?: 'module'
  readonly conditions: readonly string[]
  readonly fileResolver: (path: string) => string | false | undefined
  readonly excludedDirectory: (directory: string) => boolean
  readonly aliased: boolean
  readonly inputs?: ResolutionInputCollector
}
export interface ConfigPolicy {
  readonly purpose: 'tsconfig-extends'
  readonly inputs?: ResolutionInputCollector
  readonly configLookup?: boolean
  readonly stack?: readonly string[]
}

export function resolvePackage(
  specifier: string,
  importer: string,
  options: ModulePolicy | ConfigPolicy,
): string | false | undefined {
  const parts = specifier.split('/')
  const nameLength = specifier.startsWith('@') ? 2 : 1
  const name = parts.slice(0, nameLength).join('/')
  if (options.purpose === 'tsconfig-extends') {
    // Installed TS 6.0.3 lib/typescript.js:45327-45343,45963-46575:
    // JSON config lookup, CJS require/types/node conditions, no resolution cache.
    if (specifier === '.' || specifier === '..') {
      const directory = resolve(dirname(importer), specifier)
      return directoryConfig(
        directory,
        {
          directory,
          manifest: readPackageManifest(
            join(directory, 'package.json'),
            'tsconfig-extends',
            options.inputs,
          ),
        },
        options,
      )
    }
    const scoped = configScopeTarget(specifier, importer, options)
    if (scoped !== undefined) return scoped ?? undefined
    if (specifier.includes(':')) return undefined
  }
  let directory = dirname(importer)
  while (true) {
    const packageDir = join(directory, 'node_modules', name)
    switch (options.purpose) {
      case 'tsconfig-extends':
        if (basename(directory) !== 'node_modules') {
          const found = configPackageCandidate(
            specifier.endsWith('/') ? `${packageDir}/` : packageDir,
            parts.slice(nameLength).join('/'),
            options,
          )
          if (found) return realpathResolution(found, options.inputs)
        }
        break
      case 'module':
      case undefined: {
        if (options.excludedDirectory(packageDir)) return false
        if (isFile(join(packageDir, 'package.json'), options.inputs)) {
          const target = join(packageDir, ...parts.slice(nameLength))
          if (
            options.excludedDirectory(dirname(target)) ||
            options.excludedDirectory(target)
          )
            return false
          const manifest = readPackageManifest(
            join(packageDir, 'package.json'),
            'module',
            options.inputs,
          )
          const found = resolvePackageEntry(
            packageDir,
            manifest,
            ['.', ...parts.slice(nameLength)].join('/'),
            options,
          )
          if (
            found === undefined &&
            options.aliased &&
            manifest.exports !== undefined
          )
            throw new ModuleAliasPackageError(importer, specifier)
          return found ? realpathResolution(found, options.inputs) : found
        }
        break
      }
    }
    const parent = dirname(directory)
    if (parent === directory) return undefined
    directory = parent
  }
}

export function findPackage(
  dir: string,
  name: string,
  excludedDirectory?: (directory: string) => boolean,
  inputs?: ResolutionInputCollector,
): string | false | undefined {
  const packageDir = join(dir, 'node_modules', name)
  if (excludedDirectory?.(packageDir)) return false
  if (isFile(join(packageDir, 'package.json'), inputs)) return packageDir
  const parent = dirname(dir)
  return parent === dir
    ? undefined
    : findPackage(parent, name, excludedDirectory, inputs)
}

function resolvePackageEntry(
  dir: string,
  manifest: Record<string, unknown>,
  subpath: string,
  options: ModulePolicy,
): string | false | undefined {
  if (manifest.exports !== undefined) {
    const target = exportsTarget(manifest.exports, subpath, options.conditions)
    return typeof target !== 'string'
      ? undefined
      : options.fileResolver(join(dir, target))
  }
  if (subpath !== '.') return options.fileResolver(join(dir, subpath))
  const main = [manifest.module, manifest.main].find(
    (entry): entry is string => typeof entry === 'string',
  )
  return options.fileResolver(join(dir, main ?? 'index'))
}
