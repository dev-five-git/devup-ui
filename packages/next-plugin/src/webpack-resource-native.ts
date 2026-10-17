import type { Compiler } from 'webpack'

import { isMdxRecord, type MdxLoader } from './mdx-pipeline'

export type WebpackResourceBinding = {
  readonly compiler: Compiler
  readonly params: Parameters<
    Compiler['hooks']['beforeCompile']['callAsync']
  >[0]
  readonly effectiveConfiguration: Compiler['options']
  readonly pipelines: readonly import('./mdx-pipeline').MdxPipeline[]
}
export type NativeEffect = {
  readonly type: string
  readonly value: unknown
}
export type NativeRuleSet = {
  readonly exec: (
    facts: Readonly<Record<string, unknown>>,
  ) => readonly NativeEffect[]
}

function isRuleSet(value: unknown): value is NativeRuleSet {
  return isMdxRecord(value) && typeof value.exec === 'function'
}
export function nativeRuleSet(factory: object): NativeRuleSet {
  if (!('ruleSet' in factory) || !isRuleSet(factory.ruleSet))
    throw new TypeError(
      'receiving native NormalModuleFactory RuleSet is unavailable',
    )
  return factory.ruleSet
}

export function resourceFacts(
  binding: WebpackResourceBinding,
  filename: string,
) {
  return {
    resource: filename,
    realResource: filename,
    compiler: binding.compiler.name,
  }
}

export function compileResourceEnvelope(
  binding: WebpackResourceBinding,
  rules: readonly Readonly<Record<string, unknown>>[],
): NativeRuleSet {
  const actual = binding.params.normalModuleFactory
  const options = binding.compiler.options
  const factory: unknown = Reflect.construct(actual.constructor, [
    {
      context: binding.compiler.context,
      fs: binding.compiler.inputFileSystem,
      resolverFactory: binding.compiler.resolverFactory,
      options: { ...options.module, defaultRules: [], rules },
      associatedObjectForCache: binding.compiler.root,
      layers:
        'layers' in options.experiments
          ? options.experiments.layers
          : undefined,
    },
  ])
  if (!isMdxRecord(factory))
    throw new TypeError('native envelope factory is unavailable')
  return nativeRuleSet(factory)
}

export function nativeLoaders(
  effects: readonly NativeEffect[],
  type: string,
): readonly MdxLoader[] {
  return effects
    .filter((effect) => effect.type === type)
    .map(({ value }) => {
      if (!isMdxRecord(value) || typeof value.loader !== 'string')
        throw new TypeError('native RuleSet returned an invalid loader effect')
      const options = value.options
      if (
        options !== undefined &&
        typeof options !== 'string' &&
        !isMdxRecord(options)
      )
        throw new TypeError('native loader options are unsupported')
      return {
        loader: value.loader,
        ...(options === undefined ? {} : { options }),
        ...(typeof value.ident === 'string' ? { ident: value.ident } : {}),
      }
    })
}
