import { createHash } from 'node:crypto'

/** Rollup and Rolldown name assets like this when nothing else is set. */
const DEFAULT_ASSET_FILE_NAMES = 'assets/[name]-[hash][extname]'
const HASH_PLACEHOLDER_RE = /\[hash(?::(\d+))?\]/g
const DEFAULT_HASH_LENGTH = 8

/** The part of Rollup's and Rolldown's `PreRenderedAsset` used here. */
export interface AssetNameInfo {
  readonly name?: string
  readonly names?: readonly string[]
}

export type AssetFileNames<Info extends AssetNameInfo> =
  string | ((info: Info) => string)

function hashOf(content: string, length: string | undefined): string {
  return createHash('sha256')
    .update(content)
    .digest('base64url')
    .slice(0, length ? Number(length) : DEFAULT_HASH_LENGTH)
}

/**
 * Wraps `output.assetFileNames` so a devup-ui sheet's `[hash]` is the hash of
 * the finished sheet rather than of what Vite read when it loaded the module.
 *
 * That load races the extraction of the other members of the sheet, so the
 * hash the bundler would compute changes with module order, and misses styles
 * extracted after the load. Bundlers name an asset when it is emitted, which
 * for CSS is in `renderChunk`: every module has been extracted by then, and
 * the JS chunks that import the sheet are hashed after it.
 *
 * Any other asset keeps the name the original option gives it.
 */
export function hashSheetAssetNames<Info extends AssetNameInfo>(
  assetFileNames: AssetFileNames<Info> | undefined,
  sheetOf: (info: Info) => string | undefined,
): (info: Info) => string {
  return (info) => {
    const template =
      typeof assetFileNames === 'function'
        ? assetFileNames(info)
        : (assetFileNames ?? DEFAULT_ASSET_FILE_NAMES)
    const sheet = sheetOf(info)
    if (sheet === undefined) return template
    return template.replace(HASH_PLACEHOLDER_RE, (_match, length?: string) =>
      hashOf(sheet, length),
    )
  }
}

/** Every name the bundler gave an asset, newest API first. */
export function assetNames(info: AssetNameInfo): readonly string[] {
  return info.names ?? (info.name ? [info.name] : [])
}
