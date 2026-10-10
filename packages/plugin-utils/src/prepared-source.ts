import type { PreparedSource } from './import-graph'

export type SourceType = 'compiled-mdx'

export class PreparedSourceTypeError extends TypeError {
  constructor(cause: unknown) {
    super(
      `source type cannot use the prepared source at build time: ${cause instanceof Error ? cause.message : String(cause)}`,
      { cause },
    )
    this.name = 'PreparedSourceTypeError'
  }
}

export function readPreparedSource(prepared: PreparedSource): PreparedSource {
  if (typeof prepared !== 'object' || prepared === null) return prepared
  try {
    const sourceType: unknown = prepared.sourceType
    if (sourceType !== undefined && sourceType !== 'compiled-mdx')
      throw new TypeError('expected `compiled-mdx`')
    return {
      code: prepared.code,
      map: prepared.map,
      ...(sourceType === undefined ? {} : { sourceType }),
    }
  } catch (cause) {
    throw new PreparedSourceTypeError(cause)
  }
}
