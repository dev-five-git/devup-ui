import { extname } from 'node:path'

import { SOURCE_EXTENSIONS, SOURCE_FILE_RE } from './shared'

export type MdxSelection = boolean | readonly string[]
export interface SourceSelectionOptions {
  readonly includeMdx?: MdxSelection
}

export function sourceExtensions(selection?: MdxSelection): readonly string[] {
  return [
    ...SOURCE_EXTENSIONS,
    ...(selection === true ? ['.mdx'] : selection || []).map((extension) =>
      extension.toLowerCase(),
    ),
  ]
}

export function isSelectedSource(
  filename: string,
  selection?: MdxSelection,
): boolean {
  return (
    SOURCE_FILE_RE.test(filename) ||
    sourceExtensions(selection).includes(extname(filename).toLowerCase())
  )
}
