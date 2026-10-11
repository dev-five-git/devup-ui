import { dirname, join, resolve } from 'node:path'

import { createDirectoryExclusion } from './directory-exclusion'
import { ConfigLoadError } from './load-config'
import {
  isResolutionFile as isFile,
  readPackageManifest,
} from './packaged-config-inputs'
import {
  realpathResolution,
  type ResolutionInputCollector,
} from './resolution-inputs'
import { SOURCE_EXTENSIONS } from './shared'

export function resolveFile(
  candidateBase: string,
  options: {
    readonly extensions: readonly string[]
    readonly excludedDirectory: (directory: string) => boolean
    readonly inputs?: ResolutionInputCollector
  } = {
    extensions: SOURCE_EXTENSIONS,
    excludedDirectory: createDirectoryExclusion(),
  },
  stack: readonly string[] = [],
): string | false | undefined {
  if (
    options.excludedDirectory(dirname(candidateBase)) ||
    options.excludedDirectory(candidateBase)
  )
    return false
  if (isFile(candidateBase, options.inputs)) return resolve(candidateBase)
  for (const extension of options.extensions) {
    const candidate = `${candidateBase}${extension}`
    if (isFile(candidate, options.inputs)) return resolve(candidate)
  }
  const manifestFile = join(candidateBase, 'package.json')
  if (isFile(manifestFile, options.inputs)) {
    const canonical = realpathResolution(candidateBase, options.inputs)
    if (stack.includes(canonical))
      throw new ConfigLoadError(
        manifestFile,
        new Error(
          `Package directory entry cycle: ${[...stack, canonical].join(' -> ')}`,
        ),
      )
    const manifest = readPackageManifest(manifestFile, 'module', options.inputs)
    for (const main of [manifest.module, manifest.main]) {
      if (typeof main !== 'string' || !main || main === '.' || main === './')
        continue
      const found = resolveFile(join(candidateBase, main), options, [
        ...stack,
        canonical,
      ])
      if (found !== undefined) return found
    }
  }
  for (const extension of options.extensions) {
    const candidate = join(candidateBase, `index${extension}`)
    if (isFile(candidate, options.inputs)) return resolve(candidate)
  }
  return undefined
}

export function resolveJsonFile(
  candidate: string,
  options: {
    readonly inputs?: ResolutionInputCollector
    readonly exact?: boolean
    readonly configLookup?: boolean
  },
): string | undefined {
  const { inputs, exact = false, configLookup = true } = options
  const known = /(?:\.d\.ts|\.[^./\\]+)$/.exec(candidate)?.[0]
  if (
    known === '.json' ||
    (configLookup && (known === '.js' || known === '.ts' || known === '.d.ts'))
  ) {
    const json =
      known === '.json'
        ? candidate
        : candidate.slice(0, -known.length) + '.json'
    if (isFile(json, inputs)) return json
  }
  if (!exact && configLookup) {
    const json = `${candidate}.json`
    if (isFile(json, inputs)) return json
  }
  return undefined
}
