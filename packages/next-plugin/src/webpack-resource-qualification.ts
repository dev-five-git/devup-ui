import type { MdxLoader, MdxPipeline } from './mdx-pipeline'
import type { WebpackResourceBoundary } from './webpack-resource-error'
import {
  resourceFacts,
  type WebpackResourceBinding,
} from './webpack-resource-native'
import type { ResourceRule } from './webpack-resource-rules'

export function isDevupLoader(loader: MdxLoader): boolean {
  return /(?:^|[/\\])@devup-ui[/\\]next-plugin[/\\](?:dist[/\\])?loader(?:\.[cm]?[jt]s)?$/.test(
    loader.loader,
  )
}

export function reachableResourceRules(
  binding: WebpackResourceBinding,
  rules: readonly ResourceRule[],
  filename: string,
): readonly ResourceRule[] {
  const facts = resourceFacts(binding, filename)
  return rules
    .filter((rule) => rule.envelope.exec(facts).length > 0)
    .map((rule) => {
      const shadow = rule.controls.find(
        (control) =>
          control.unknownFact && control.envelope.exec(facts).length > 0,
      )
      return shadow ? Object.freeze({ ...rule, shadow }) : rule
    })
}

export function resourceBoundary(
  filename: string,
  rule: Pick<ResourceRule, 'position' | 'test' | 'unknownFact'>,
  reason: string,
): WebpackResourceBoundary {
  return Object.freeze({
    filename,
    rulePosition: rule.position,
    test: rule.test,
    unknownFact: rule.unknownFact ?? 'input-producing loader semantics',
    reason,
  })
}

export function qualifyMdxRules(
  filename: string,
  reachable: readonly ResourceRule[],
  pipelines: readonly MdxPipeline[],
): {
  readonly pipeline?: MdxPipeline
  readonly boundary?: WebpackResourceBoundary
  readonly downstream: readonly MdxLoader[]
} {
  const candidates = pipelines.filter(
    (pipeline) =>
      pipeline.bundler === 'webpack' &&
      reachable.some((rule) => rule.key === pipeline.ruleKey),
  )
  if (candidates.length === 0) return { downstream: [] }
  const [pipeline] = candidates
  if (!pipeline) throw new TypeError('native pipeline candidate disappeared')
  const selectedIndex = reachable.findIndex(
    (rule) => rule.key === pipeline.ruleKey,
  )
  const selected = reachable[selectedIndex]
  if (!selected) throw new TypeError('native rule candidate disappeared')
  if (selected.shadow)
    return {
      boundary: resourceBoundary(
        filename,
        selected.shadow,
        'an earlier oneOf branch can suppress the input-producing MDX segment',
      ),
      downstream: [],
    }
  if (
    candidates.length > 1 ||
    selected.unknownFact ||
    selected.stage !== 'normal' ||
    pipeline.issue ||
    pipeline.loaders.length !== 1
  )
    return {
      boundary: resourceBoundary(
        filename,
        candidates.length > 1
          ? (reachable.find(
              (rule) =>
                rule.key !== pipeline.ruleKey &&
                candidates.some((item) => item.ruleKey === rule.key),
            ) ?? selected)
          : selected,
        'the resource can select a competing/contextual or unsupported pre-Devup MDX segment',
      ),
      downstream: [],
    }
  const downstream: MdxLoader[] = []
  for (const [index, rule] of reachable.entries()) {
    if (rule.dynamic)
      return {
        boundary: {
          ...resourceBoundary(
            filename,
            rule,
            'a loader factory can change input/options or pitch behavior',
          ),
          unknownFact: 'use(context)',
        },
        downstream: [],
      }
    if (
      rule.stage === 'pre' ||
      (rule.stage === 'normal' && index > selectedIndex)
    )
      return {
        boundary: resourceBoundary(
          filename,
          rule,
          'an additional upstream segment can produce different bytes before Devup',
        ),
        downstream: [],
      }
    const devup = rule.loaders.findIndex(isDevupLoader)
    downstream.push(
      ...(index === selectedIndex
        ? rule.loaders.slice(0, devup)
        : rule.loaders),
    )
  }
  return { pipeline, downstream: Object.freeze(downstream) }
}

export type WebpackOrdinaryEligibility =
  | { readonly kind: 'disk-first' }
  | ({ readonly kind: 'native-required' } & WebpackResourceBoundary)

export function qualifyOrdinaryRules(
  filename: string,
  reachable: readonly ResourceRule[],
): {
  readonly eligibility: WebpackOrdinaryEligibility
  readonly downstream: readonly MdxLoader[]
} {
  const pre = reachable.filter((rule) => rule.stage === 'pre')
  const selectedIndex = pre.findLastIndex((rule) =>
    rule.loaders.some(isDevupLoader),
  )
  const selected = pre[selectedIndex]
  if (!selected)
    return {
      eligibility: {
        kind: 'native-required',
        filename,
        rulePosition: 'module.rules',
        test: '<native>',
        unknownFact: 'first-input boundary',
        reason: 'no unconditional native Devup pre boundary is available',
      },
      downstream: [],
    }
  const upstream = pre
    .slice(selectedIndex + 1)
    .find((rule) => rule.loaders.length || rule.dynamic)
  const issue =
    selected.shadow ??
    (selected.unknownFact || selected.dynamic ? selected : upstream)
  const index = selected.loaders.findIndex(isDevupLoader)
  if (issue || index !== selected.loaders.length - 1)
    return {
      eligibility: {
        kind: 'native-required',
        ...resourceBoundary(
          filename,
          issue ?? selected,
          'the first input can depend on another pre-loader or missing origin facts',
        ),
      },
      downstream: [],
    }
  const dynamic = reachable.find((rule) => rule.dynamic)
  if (dynamic)
    return {
      eligibility: {
        kind: 'native-required',
        ...resourceBoundary(
          filename,
          dynamic,
          'an unobserved loader factory can replace disk input',
        ),
      },
      downstream: [],
    }
  return {
    eligibility: Object.freeze({ kind: 'disk-first' }),
    downstream: Object.freeze(
      reachable.flatMap((rule) =>
        rule === selected ? rule.loaders.slice(0, index) : rule.loaders,
      ),
    ),
  }
}
