export type MdxRule = Readonly<Record<string, unknown>>
export type MdxLoader = {
  readonly loader: string
  readonly options?: string | Readonly<Record<string, unknown>>
  readonly ident?: string
}
export type MdxPipeline = {
  readonly bundler: 'turbo' | 'webpack'
  readonly ruleKey: string
  readonly conditions: readonly MdxRule[]
  readonly aliases: Readonly<Record<string, unknown>>
  readonly loaders: readonly MdxLoader[]
  readonly issue?: string
}
export type MdxRuleSet =
  | {
      readonly bundler: 'turbo'
      readonly rules: Readonly<Record<string, unknown>>
      readonly aliases: Readonly<Record<string, unknown>>
    }
  | {
      readonly bundler: 'webpack'
      readonly rules: readonly unknown[]
      readonly aliases: Readonly<Record<string, unknown>>
    }

export function isMdxRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function loaderDescriptor(value: unknown): MdxLoader | undefined {
  if (typeof value === 'string') return { loader: value }
  if (!isMdxRecord(value) || typeof value.loader !== 'string') return
  if (
    value.options !== undefined &&
    typeof value.options !== 'string' &&
    !isMdxRecord(value.options)
  )
    return
  if (value.ident !== undefined && typeof value.ident !== 'string') return
  return {
    loader: value.loader,
    ...(value.options === undefined ? {} : { options: value.options }),
    ...(value.ident === undefined ? {} : { ident: value.ident }),
  }
}

function isCompiler(loader: string): boolean {
  return (
    /(?:^|\/)@next\/mdx\/mdx-js-loader(?:\.js)?$/.test(
      loader.replaceAll('\\', '/'),
    ) ||
    /(?:^|\/)@mdx-js\/loader(?:\/index\.(?:js|cjs)|\/lib\/index\.js)?$/.test(
      loader.replaceAll('\\', '/'),
    )
  )
}

export class MdxPipelineError extends Error {
  readonly name = 'MdxPipelineError'
  constructor(
    readonly filename: string,
    readonly detail: string,
  ) {
    super(
      `${filename}:1:1: devup-ui MDX preparation cannot use \`${filename}\` at build time: ${detail}; needs DevupUI outermost or Next 16.2+ with a recognized configured MDX pipeline`,
    )
  }
}

export function requireMdxPipeline(
  filename: string,
  pipeline: MdxPipeline | undefined,
): MdxPipeline {
  if (!pipeline || pipeline.issue || pipeline.loaders.length === 0) {
    throw new MdxPipelineError(
      filename,
      pipeline?.issue ?? 'configured MDX compiler chain is unavailable',
    )
  }
  return pipeline
}

export function composeMdxRules(input: MdxRuleSet, extraction: MdxLoader) {
  const pipelines: MdxPipeline[] = []
  function compose(
    rule: unknown,
    key: string,
    parents: readonly MdxRule[],
  ): unknown {
    if (!isMdxRecord(rule)) return rule
    const conditions = [...parents, rule]
    const member = input.bundler === 'turbo' ? 'loaders' : 'use'
    const chain = rule[member]
    let composed = rule
    if (Array.isArray(chain)) {
      const normalized = chain.filter(
        (value) => loaderDescriptor(value)?.loader !== extraction.loader,
      )
      const descriptors = normalized.map(loaderDescriptor)
      const compilerIndex = descriptors.findIndex(
        (loader) => loader && isCompiler(loader.loader),
      )
      if (compilerIndex >= 0) {
        const segment = descriptors.slice(compilerIndex)
        const loaders = segment.filter(
          (loader): loader is MdxLoader => loader !== undefined,
        )
        const forbidden = loaders.some((loader) =>
          /(?:@devup-ui|next-swc-loader|next-flight-loader)/.test(
            loader.loader,
          ),
        )
        const issue = segment.includes(undefined)
          ? 'malformed compiler loader options'
          : forbidden
            ? 'pre-extraction chain contains Devup, SWC or Flight'
            : loaders.filter((loader) => isCompiler(loader.loader)).length !== 1
              ? 'multiple MDX compilers in one chain'
              : undefined
        pipelines.push({
          bundler: input.bundler,
          ruleKey: key,
          conditions,
          aliases: input.aliases,
          loaders,
          ...(issue ? { issue } : {}),
        })
        if (!issue) {
          composed = {
            ...rule,
            [member]: [
              ...normalized.slice(0, compilerIndex),
              extraction,
              ...normalized.slice(compilerIndex),
            ],
          }
        }
      }
    }
    for (const nested of ['rules', 'oneOf']) {
      if (Array.isArray(rule[nested])) {
        composed = {
          ...composed,
          [nested]: rule[nested].map((child, index) =>
            compose(child, `${key}.${nested}.${index}`, conditions),
          ),
        }
      }
    }
    return composed
  }
  switch (input.bundler) {
    case 'turbo':
      return {
        rules: Object.fromEntries(
          Object.entries(input.rules).map(([key, value]) => [
            key,
            Array.isArray(value)
              ? value.map((rule, index) => compose(rule, `${key}.${index}`, []))
              : compose(value, key, []),
          ]),
        ),
        pipelines,
      }
    case 'webpack':
      return {
        rules: input.rules.map((rule, index) =>
          compose(rule, String(index), []),
        ),
        pipelines,
      }
    default:
      return input satisfies never
  }
}
