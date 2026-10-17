import { statSync } from 'node:fs'

import { isRecord, parseJsonc } from './jsonc'
import { ConfigLoadError } from './load-config'
import {
  readResolutionFile,
  recordResolutionFailure,
  type ResolutionInputCollector,
} from './resolution-inputs'

export function isResolutionFile(
  path: string,
  inputs?: ResolutionInputCollector,
): boolean {
  try {
    const file = statSync(path).isFile()
    if (file) inputs?.file(path)
    return file
  } catch (cause) {
    recordResolutionFailure(path, cause, inputs)
    if (
      cause instanceof Error &&
      'code' in cause &&
      (cause.code === 'ENOENT' || cause.code === 'ENOTDIR')
    )
      return false
    throw cause
  }
}

export function readPackageManifest(
  file: string,
  purpose: 'module' | 'tsconfig-extends',
  inputs?: ResolutionInputCollector,
): Record<string, unknown> {
  // TypeScript 6.0.3 lib/typescript.js:21164-21184 reads JSON/JSONC, parse misses => {}.
  // The @typescript/typescript6 6.0.2 wrapper reexports this installed compiler.
  switch (purpose) {
    case 'module':
      try {
        const manifest: unknown = JSON.parse(readResolutionFile(file, inputs))
        if (!isRecord(manifest))
          throw new TypeError('Expected a package manifest object')
        return manifest
      } catch (cause) {
        throw new ConfigLoadError(file, cause)
      }
    case 'tsconfig-extends':
      if (!isResolutionFile(file, inputs)) return {}
      try {
        const source = readResolutionFile(file, inputs)
        let manifest: unknown
        try {
          manifest = JSON.parse(source)
        } catch (cause) {
          if (!(cause instanceof SyntaxError)) throw cause
          manifest = parseJsonc(source, true)
        }
        return isRecord(manifest) ? manifest : {}
      } catch (cause) {
        if (cause instanceof SyntaxError) return {}
        throw cause
      }
  }
}
