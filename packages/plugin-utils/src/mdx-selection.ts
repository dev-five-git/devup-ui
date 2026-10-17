import { extname } from 'node:path'

import { sourceExtensions } from './source-selection'

export class MdxExtensionError extends Error {
  readonly name = 'MdxExtensionError'
  constructor(readonly extension: string) {
    super(
      `mdxExtensions:1:1: invalid extension ${JSON.stringify(extension)}; use leading-dot literal extensions, for example '.mdown'`,
    )
  }
}

export function normalizeMdxExtensions(
  extensions: readonly string[] = ['.mdx'],
): readonly string[] {
  return [
    ...new Set(
      extensions.map((extension) => {
        if (!/^\.[a-z\d][a-z\d_-]*$/i.test(extension))
          throw new MdxExtensionError(extension)
        return extension.toLowerCase()
      }),
    ),
  ]
}

export function isMdxSource(
  filename: string,
  extensions: readonly string[],
): boolean {
  return extensions.includes(extname(filename.split('?')[0]).toLowerCase())
}

export function mdxSourceFilter(extensions: readonly string[]): RegExp {
  return extensions.length
    ? new RegExp(
        `\\.${extensions.length === 1 ? extensions[0].slice(1) : `(?:${extensions.map((extension) => extension.slice(1)).join('|')})`}$`,
        'i',
      )
    : /$^/
}

export function selectedSourceFilter(extensions: readonly string[]): RegExp {
  return mdxSourceFilter(sourceExtensions(extensions))
}
