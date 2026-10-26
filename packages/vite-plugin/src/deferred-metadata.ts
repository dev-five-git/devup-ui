export interface DeferredDefault {
  readonly extraction: string
  readonly request: string
  readonly exports: readonly string[]
  readonly line: number
  readonly column: number
  readonly demand: 'render-only'
  readonly attributes: boolean
  readonly cutImporter: string
  readonly cutRequest: string
  readonly cutExport: 'default'
}

export function deferredDefaults(
  serialized: string,
): readonly DeferredDefault[] {
  const value: unknown = JSON.parse(serialized)
  if (!Array.isArray(value))
    throw new TypeError('Invalid deferred default metadata')
  return value.map((item: unknown): DeferredDefault => {
    if (
      !item ||
      typeof item !== 'object' ||
      !('extraction' in item) ||
      typeof item.extraction !== 'string' ||
      !('request' in item) ||
      typeof item.request !== 'string' ||
      !('exports' in item) ||
      !Array.isArray(item.exports) ||
      !item.exports.every((name: unknown) => typeof name === 'string') ||
      !('line' in item) ||
      typeof item.line !== 'number' ||
      !Number.isSafeInteger(item.line) ||
      item.line < 1 ||
      !('column' in item) ||
      typeof item.column !== 'number' ||
      !Number.isSafeInteger(item.column) ||
      item.column < 1 ||
      !('demand' in item) ||
      item.demand !== 'render-only' ||
      !('attributes' in item) ||
      typeof item.attributes !== 'boolean' ||
      !('cutImporter' in item) ||
      typeof item.cutImporter !== 'string' ||
      !('cutRequest' in item) ||
      typeof item.cutRequest !== 'string' ||
      !('cutExport' in item) ||
      item.cutExport !== 'default'
    )
      throw new TypeError('Invalid deferred default record')
    return {
      extraction: item.extraction,
      request: item.request,
      exports: item.exports,
      line: item.line,
      column: item.column,
      demand: item.demand,
      attributes: item.attributes,
      cutImporter: item.cutImporter,
      cutRequest: item.cutRequest,
      cutExport: item.cutExport,
    }
  })
}

export class DeferredDefaultError extends Error {
  readonly name = 'DeferredDefaultError'
  constructor(
    readonly obligation: DeferredDefault,
    readonly reason:
      | 'outer-order-unproven'
      | 'outer-result-unproven'
      | 'forwarding-unproven'
      | 'mdx-pipeline-unproven'
      | 'metadata-unproven',
    diagnosticPrefix = `${obligation.extraction}:${obligation.line}:${obligation.column}`,
  ) {
    super(
      `${diagnosticPrefix}: deferred default cannot use \`${obligation.request}\` at build time: ${reason}; the actual native target must complete this build's owned MDX pipeline with an ordinary local callable default; move values into JS/TS`,
    )
  }
}

export class DeferredMetadataError extends Error {
  readonly name = 'DeferredMetadataError'
  constructor(
    readonly filename: string,
    options?: ErrorOptions,
  ) {
    super(
      `${filename}:1:1: deferred default metadata cannot use missing or changed obligations at build time: the exact native module must retain its own extraction metadata`,
      options,
    )
  }
}
