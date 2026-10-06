import { extname } from 'node:path'

import type { MdxNativeLoaderFacts } from './mdx-binding'
import { snapshotMdxBindingValue } from './mdx-binding-value'
import type { MdxLoader, MdxPipeline } from './mdx-pipeline'
import {
  type MdxPreparationContext,
  recognizeMdxPrewarmStep,
} from './mdx-prewarm-boundary'
import { createWebpackLoaderFacts } from './webpack-mdx-facts'
import { compilerResourceBoundary } from './webpack-resource-compiler'
import {
  webpackResourceDelivery,
  webpackResourceResolver,
} from './webpack-resource-delivery'
import { WebpackResourceError } from './webpack-resource-error'
import {
  nativeLoaders,
  nativeRuleSet,
  resourceFacts,
  type WebpackResourceBinding,
} from './webpack-resource-native'
import {
  isDevupLoader,
  qualifyMdxRules,
  qualifyOrdinaryRules,
  reachableResourceRules,
  type WebpackOrdinaryEligibility,
} from './webpack-resource-qualification'
import { collectResourceRules } from './webpack-resource-rules'

export type WebpackResourceSelection = {
  readonly pipeline: MdxPipeline
  readonly context: MdxPreparationContext
  readonly identity: ReturnType<typeof snapshotMdxBindingValue>
  readonly loaders: readonly MdxNativeLoaderFacts[]
}
export type WebpackResourceSelectorInput = {
  readonly binding: WebpackResourceBinding
  readonly configFile: string
  readonly owner: object
  readonly generation: object
}

export async function createWebpackResourceSelector(
  input: WebpackResourceSelectorInput,
) {
  const { binding, configFile } = input
  const native = nativeRuleSet(binding.params.normalModuleFactory)
  const rules = collectResourceRules(binding)
  const delivery = webpackResourceDelivery(binding)
  const resolver = webpackResourceResolver(binding)
  const facts = createWebpackLoaderFacts(binding, configFile)
  const prepared = new Map<
    MdxPipeline,
    {
      readonly pipeline: MdxPipeline
      readonly loaders: readonly MdxNativeLoaderFacts[]
      readonly identity: ReturnType<typeof snapshotMdxBindingValue>
    }
  >()
  const ordinary = new Map<string, WebpackOrdinaryEligibility>()
  async function qualifyDiskFirst(
    filename: string,
    downstream: readonly MdxLoader[],
    signal: AbortSignal,
  ): Promise<WebpackOrdinaryEligibility> {
    const pre = nativeLoaders(
      native.exec(resourceFacts(binding, filename)),
      'use-pre',
    )
    const first = pre.at(-1)
    if (!first || !isDevupLoader(first))
      return Object.freeze({
        kind: 'native-required',
        filename,
        rulePosition: 'module.rules',
        test: '<native>',
        unknownFact: 'selected first-input boundary',
        reason:
          'the actual receiving RuleSet does not put Devup at the first ordinary normal input position',
      })
    const boundary = compilerResourceBoundary(binding, filename)
    if (boundary) return Object.freeze({ kind: 'native-required', ...boundary })
    await facts.downstreamSafe(downstream, filename, signal)
    signal.throwIfAborted()
    return Object.freeze({ kind: 'disk-first' })
  }
  return Object.freeze({
    ...resolver,
    watchInputs: facts.watchInputs,
    async selectPipeline(
      filename: string,
      signal: AbortSignal,
    ): Promise<WebpackResourceSelection | undefined> {
      signal.throwIfAborted()
      const reachable = reachableResourceRules(binding, rules, filename)
      const qualification = qualifyMdxRules(
        filename,
        reachable,
        binding.pipelines,
      )
      if (qualification.boundary)
        throw new WebpackResourceError(configFile, qualification.boundary)
      if (!qualification.pipeline) return
      const original = qualification.pipeline
      if (!['.md', '.mdx'].includes(extname(filename)))
        throw new WebpackResourceError(configFile, {
          filename,
          rulePosition: original.ruleKey,
          test: '<native MDX>',
          unknownFact: 'extraction source type',
          reason: `compiled MDX under extension ${extname(filename) || '<none>'} is not supported yet (SourceType7f is unannounced); use .md or .mdx`,
        })
      const effects = native.exec(resourceFacts(binding, filename))
      const normal = nativeLoaders(effects, 'use')
      const devup = normal.findLastIndex(isDevupLoader)
      const upstream = normal.slice(devup + 1)
      if (
        devup < 0 ||
        upstream.length !== original.loaders.length ||
        upstream.some(
          (loader, index) =>
            loader.loader !== original.loaders[index]?.loader ||
            loader.options !== original.loaders[index]?.options,
        )
      )
        throw new WebpackResourceError(configFile, {
          filename,
          rulePosition: original.ruleKey,
          test: '<native>',
          unknownFact: 'selected native chain',
          reason:
            'receiving RuleSet does not select the qualified original MDX segment',
        })
      const compilerBoundary = compilerResourceBoundary(binding, filename)
      if (compilerBoundary)
        throw new WebpackResourceError(configFile, compilerBoundary)
      let result = prepared.get(original)
      if (!result) {
        const outputs = await Promise.all(
          upstream.map((loader) => facts.resolveLoader(loader, signal)),
        )
        if (
          outputs.some(
            (output) => !recognizeMdxPrewarmStep(output.facts.resolvedPath),
          )
        )
          throw new WebpackResourceError(configFile, {
            filename,
            rulePosition: original.ruleKey,
            test: '<native>',
            unknownFact: 'installed compiler semantics',
            reason:
              'the exact installed preparation5b compiler is not recognized',
          })
        const originalFacts = Object.freeze(
          outputs.map((output) => output.facts).reverse(),
        )
        result = Object.freeze({
          pipeline: Object.freeze({
            ...original,
            loaders: Object.freeze(outputs.map((output) => output.loader)),
          }),
          loaders: originalFacts,
          identity: snapshotMdxBindingValue(originalFacts),
        })
        prepared.set(original, result)
      }
      await facts.downstreamSafe(qualification.downstream, filename, signal)
      signal.throwIfAborted()
      return Object.freeze({
        ...result,
        context: Object.freeze({
          owner: input.owner,
          generation: input.generation,
          compiler: binding.compiler,
          ...delivery,
        }),
      })
    },
    ordinaryEligibility(filename: string): WebpackOrdinaryEligibility {
      const cached = ordinary.get(filename)
      if (cached) return cached
      const result = qualifyOrdinaryRules(
        filename,
        reachableResourceRules(binding, rules, filename),
      ).eligibility
      switch (result.kind) {
        case 'native-required':
          return result
        case 'disk-first':
          return Object.freeze({
            kind: 'native-required',
            filename,
            rulePosition: 'module.rules',
            test: '<native>',
            unknownFact: 'unqualified first-input boundary',
            reason:
              'await qualifyOrdinary before initial ordinary replay; this is not an attempt proof',
          })
        default:
          return result satisfies never
      }
    },
    async qualifyOrdinary(
      filename: string,
      signal: AbortSignal,
    ): Promise<WebpackOrdinaryEligibility> {
      signal.throwIfAborted()
      const qualification = qualifyOrdinaryRules(
        filename,
        reachableResourceRules(binding, rules, filename),
      )
      switch (qualification.eligibility.kind) {
        case 'native-required':
          ordinary.set(filename, qualification.eligibility)
          return qualification.eligibility
        case 'disk-first':
          break
        default:
          return qualification.eligibility satisfies never
      }
      const eligibility = await qualifyDiskFirst(
        filename,
        qualification.downstream,
        signal,
      )
      signal.throwIfAborted()
      ordinary.set(filename, eligibility)
      return eligibility
    },
  })
}
