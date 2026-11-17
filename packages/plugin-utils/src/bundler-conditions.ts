/** The part of webpack's and Rspack's `resolve` options naming `exports` conditions. */
export interface ConditionResolveOptions {
  readonly conditionNames?: readonly string[]
  readonly byDependency?: Readonly<
    Record<string, { readonly conditionNames?: readonly string[] } | undefined>
  >
}

/**
 * The `exports` conditions webpack and Rspack resolve an ESM import with:
 * those of `byDependency.esm`, where `...` stands for the general
 * `conditionNames`. `undefined` when the options name none.
 */
export function esmImportConditions(
  resolve: ConditionResolveOptions | undefined,
): string[] | undefined {
  const general = resolve?.conditionNames ?? []
  return (
    resolve?.byDependency?.esm?.conditionNames ?? resolve?.conditionNames
  )?.flatMap((condition) => (condition === '...' ? general : [condition]))
}

/** The `resolve` options of Vite naming `exports` conditions. */
export interface ViteConditionConfig {
  readonly isProduction?: boolean
  readonly resolve?: { readonly conditions?: readonly string[] }
}

/**
 * The `exports` conditions Vite resolves an import with: `import` and the
 * configured `resolve.conditions`, with `development|production` standing for
 * the current mode. `undefined` when the config names none.
 */
export function viteImportConditions(
  config: ViteConditionConfig | undefined,
): string[] | undefined {
  const mode = config?.isProduction ? 'production' : 'development'
  return (
    config?.resolve?.conditions && [
      'import',
      ...config.resolve.conditions.map((condition) =>
        condition === 'development|production' ? mode : condition,
      ),
    ]
  )
}
