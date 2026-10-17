import { createRequire } from 'node:module'
import { join } from 'node:path'

import { isMdxRecord } from './mdx-pipeline'

export type MdxJsonParser = (value: string) => unknown
function isParser(value: unknown): value is MdxJsonParser {
  return typeof value === 'function'
}

export function installedMdxJsonParser(
  root: string,
): MdxJsonParser | undefined {
  try {
    const project = createRequire(join(root, 'package.json'))
    const native = createRequire(
      project.resolve('next/dist/compiled/webpack/webpack'),
    )
    const metadata: unknown = native(
      'json-parse-even-better-errors/package.json',
    )
    if (!isMdxRecord(metadata) || metadata.version !== '3.0.2') return
    const parser: unknown = native('json-parse-even-better-errors')
    if (isParser(parser)) return parser
  } catch (cause) {
    if (!(cause instanceof Error)) throw cause
    return undefined
  }
}
