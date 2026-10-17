import { isAbsolute, join, relative, resolve, sep } from 'node:path'

import { isRecord } from './jsonc'
import { resolveJsonFile } from './owned-file-resolution'
import type { ConfigPolicy } from './owned-module-resolution'
import {
  isResolutionFile as isFile,
  readPackageManifest,
} from './packaged-config-inputs'
import { configExport } from './tsconfig-export-policy'
import { matchesTypeScriptVersion } from './typescript-version-range'

interface PackageConfigManifest {
  readonly directory: string
  readonly manifest: Record<string, unknown>
}

// Installed TS 6.0.3:46566 uses tryGetExtensionFromPath2 (22743,22822-22823).
const recognizedExtension = /\.(?:json|js|jsx|mjs|cjs|ts|tsx|mts|cts)$/

function versionPaths(
  manifest: Record<string, unknown>,
): Record<string, unknown> | undefined {
  if (!isRecord(manifest.typesVersions)) return undefined
  for (const [range, paths] of Object.entries(manifest.typesVersions))
    if (matchesTypeScriptVersion(range))
      return isRecord(paths) ? paths : undefined
  return undefined
}

function mappedPath(
  request: string,
  paths: Record<string, unknown>,
  load: (target: string) => string | undefined,
): string | null | undefined {
  let key = Object.hasOwn(paths, request) ? request : undefined
  let matched = ''
  if (key === undefined) {
    let length = -1
    for (const pattern of Object.keys(paths)) {
      const star = pattern.indexOf('*')
      if (star === -1 || star !== pattern.lastIndexOf('*')) continue
      const prefix = pattern.slice(0, star)
      const suffix = pattern.slice(star + 1)
      if (
        star > length &&
        request.startsWith(prefix) &&
        request.endsWith(suffix) &&
        request.length >= prefix.length + suffix.length
      ) {
        key = pattern
        length = star
        matched = request.slice(star, request.length - suffix.length)
      }
    }
  }
  if (key === undefined) return undefined
  const targets = paths[key]
  if (Array.isArray(targets))
    for (const target of targets) {
      if (typeof target !== 'string') continue
      const found = load(matched ? target.replace('*', matched) : target)
      if (found) return found
    }
  return null
}

export function directoryConfig(
  directory: string,
  packageConfig: PackageConfigManifest,
  options: ConfigPolicy,
): string | undefined {
  const { manifest } = packageConfig
  const field =
    options.configLookup !== false &&
    relative(packageConfig.directory, directory) === ''
      ? manifest.tsconfig
      : undefined
  const packageFile =
    typeof field === 'string' && field ? resolve(directory, field) : undefined
  const load = (candidate: string) =>
    resolveJsonFile(candidate, options) ??
    resolveJsonFile(
      join(candidate, options.configLookup === false ? 'index' : 'tsconfig'),
      options,
    )
  const paths = versionPaths(manifest)
  const fieldPath = packageFile ? relative(directory, packageFile) : ''
  if (
    paths &&
    fieldPath !== '..' &&
    !fieldPath.startsWith(`..${sep}`) &&
    !isAbsolute(fieldPath)
  ) {
    const found = mappedPath(
      relative(
        directory,
        packageFile ??
          join(
            directory,
            options.configLookup === false ? 'index' : 'tsconfig',
          ),
      ).replaceAll('\\', '/'),
      paths,
      (target) => {
        const candidate = resolve(directory, target)
        return recognizedExtension.test(target) &&
          isFile(candidate, options.inputs)
          ? candidate
          : load(candidate)
      },
    )
    if (found !== undefined) return found ?? undefined
  }
  return (
    (packageFile ? load(packageFile) : undefined) ??
    resolveJsonFile(
      join(directory, options.configLookup === false ? 'index' : 'tsconfig'),
      options,
    )
  )
}

export function configPackageCandidate(
  directory: string,
  rest: string,
  options: ConfigPolicy,
): string | undefined {
  const candidate = join(directory, rest)
  const nestedFile = join(candidate, 'package.json')
  const nestedExists = isFile(nestedFile, options.inputs)
  const nested = nestedExists
    ? readPackageManifest(nestedFile, 'tsconfig-extends', options.inputs)
    : {}
  const manifest = rest
    ? readPackageManifest(
        join(directory, 'package.json'),
        'tsconfig-extends',
        options.inputs,
      )
    : nested
  if (rest && nestedExists && !Object.hasOwn(manifest, 'exports'))
    return (
      resolveJsonFile(candidate, options) ??
      directoryConfig(
        candidate,
        { directory: candidate, manifest: nested },
        options,
      )
    )
  if (manifest.exports)
    return (
      configExport(manifest.exports, rest ? `./${rest}` : '.', {
        ...options,
        directory,
      }) ?? undefined
    )
  const load = (target: string) =>
    resolveJsonFile(target, options) ??
    directoryConfig(target, { directory, manifest }, options)
  const paths = rest ? versionPaths(manifest) : undefined
  if (paths) {
    const mapped = mappedPath(rest, paths, (target) => {
      const path = resolve(directory, target)
      return recognizedExtension.test(target) && isFile(path, options.inputs)
        ? path
        : load(path)
    })
    if (mapped !== undefined) return mapped ?? undefined
  }
  return load(candidate)
}
