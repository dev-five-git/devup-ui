import type { Configuration, RuleSetRule } from 'webpack'

import { composeMdxRules, type MdxLoader } from './mdx-pipeline'

type NestedRule = NonNullable<RuleSetRule['rules']>[number]

/** Keep webpack's typed rule shell; only the approved MDX use segment changes. */
export function composeWebpackMdxRules(
  config: Configuration,
  extraction: MdxLoader,
) {
  const rules = config.module?.rules ?? []
  const alias = config.resolve?.alias ?? {}
  const aliases = Array.isArray(alias)
    ? Object.fromEntries(
        alias.map(({ name, alias, onlyModule }) => [
          `${name}${onlyModule ? '$' : ''}`,
          alias,
        ]),
      )
    : alias
  const { pipelines } = composeMdxRules(
    { bundler: 'webpack', rules, aliases },
    extraction,
  )
  const byKey = new Map(
    pipelines.map((pipeline) => [pipeline.ruleKey, pipeline]),
  )
  function compose(rule: NestedRule, key: string): NestedRule {
    if (!rule || typeof rule === 'string') return rule
    const pipeline = byKey.get(key)
    let result: RuleSetRule = rule
    if (pipeline && !pipeline.issue && Array.isArray(rule.use)) {
      const normalized = rule.use.filter((item) =>
        typeof item === 'string'
          ? item !== extraction.loader
          : typeof item === 'object' && item !== null
            ? item.loader !== extraction.loader
            : true,
      )
      const index = normalized.length - pipeline.loaders.length
      result = {
        ...result,
        use: [
          ...normalized.slice(0, index),
          extraction,
          ...normalized.slice(index),
        ],
      }
    }
    if (rule.rules) {
      result = {
        ...result,
        rules: rule.rules.map((child, index) =>
          compose(child, `${key}.rules.${index}`),
        ),
      }
    }
    if (rule.oneOf) {
      result = {
        ...result,
        oneOf: rule.oneOf.map((child, index) =>
          compose(child, `${key}.oneOf.${index}`),
        ),
      }
    }
    return result
  }
  return {
    rules: rules.map((rule, index) =>
      rule === '...' ? rule : compose(rule, String(index)),
    ),
    pipelines,
  }
}
