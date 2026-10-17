import { isMdxRecord, type MdxLoader, type MdxRule } from './mdx-pipeline'
import {
  createResourceEnvelope,
  resourceRuleLabel,
  unknownResourceFact,
} from './webpack-resource-envelope'
import type {
  NativeRuleSet,
  WebpackResourceBinding,
} from './webpack-resource-native'

export type ResourceControl = {
  readonly position: string
  readonly test: string
  readonly unknownFact: string | undefined
  readonly envelope: NativeRuleSet
}

export type ResourceRule = {
  readonly position: string
  readonly key: string
  readonly rules: readonly MdxRule[]
  readonly rule: MdxRule
  readonly envelope: NativeRuleSet
  readonly loaders: readonly MdxLoader[]
  readonly dynamic: boolean
  readonly unknownFact: string | undefined
  readonly stage: 'pre' | 'post' | 'normal'
  readonly test: string
  readonly controls: readonly ResourceControl[]
  readonly shadow?: ResourceControl
}

type RuleTrace = {
  readonly key: string
  readonly parents: readonly MdxRule[]
  readonly controls: readonly ResourceControl[]
}

function descriptors(rule: MdxRule): readonly MdxLoader[] | undefined {
  const use =
    rule.use ??
    (rule.loader === undefined
      ? []
      : [{ loader: rule.loader, options: rule.options }])
  const values: readonly unknown[] = Array.isArray(use) ? use : [use]
  const result: MdxLoader[] = []
  for (const value of values) {
    if (typeof value === 'string') result.push({ loader: value })
    else if (
      isMdxRecord(value) &&
      typeof value.loader === 'string' &&
      (value.options === undefined ||
        typeof value.options === 'string' ||
        isMdxRecord(value.options))
    ) {
      result.push({
        loader: value.loader,
        ...(value.options === undefined ? {} : { options: value.options }),
      })
    } else return
  }
  return result
}

export function collectResourceRules(
  binding: WebpackResourceBinding,
): readonly ResourceRule[] {
  const result: ResourceRule[] = []
  function visit(rule: unknown, trace: RuleTrace) {
    if (!isMdxRecord(rule)) return
    const { key, parents, controls } = trace
    const path = [...parents, rule]
    if (rule.use !== undefined || rule.loader !== undefined) {
      const loaders = descriptors(rule)
      result.push(
        Object.freeze({
          ...resourceRuleLabel(key, rule),
          key,
          rules: Object.freeze(path),
          rule,
          envelope: createResourceEnvelope(binding, path),
          loaders: Object.freeze(loaders ?? []),
          dynamic: loaders === undefined,
          unknownFact: unknownResourceFact(path),
          stage:
            rule.enforce === 'pre'
              ? 'pre'
              : rule.enforce === 'post'
                ? 'post'
                : 'normal',
          controls: Object.freeze([...controls]),
        }),
      )
    }
    for (const member of ['rules', 'oneOf']) {
      const children = rule[member]
      const previous: ResourceControl[] = []
      if (Array.isArray(children))
        children.forEach((child: unknown, index: number) => {
          const childKey = `${key}.${member}.${index}`
          visit(child, {
            key: childKey,
            parents: path,
            controls:
              member === 'oneOf' ? [...controls, ...previous] : controls,
          })
          if (member === 'oneOf' && isMdxRecord(child)) {
            previous.push(
              Object.freeze({
                ...resourceRuleLabel(childKey, child),
                unknownFact: unknownResourceFact([...path, child]),
                envelope: createResourceEnvelope(binding, [...path, child]),
              }),
            )
          }
        })
    }
  }
  binding.effectiveConfiguration.module.defaultRules.forEach((rule, index) =>
    visit(rule, { key: `defaultRules.${index}`, parents: [], controls: [] }),
  )
  binding.effectiveConfiguration.module.rules.forEach((rule, index) =>
    visit(rule, { key: String(index), parents: [], controls: [] }),
  )
  return Object.freeze(result)
}
