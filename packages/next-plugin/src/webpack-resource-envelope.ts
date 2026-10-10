import { isMdxRecord, type MdxRule } from './mdx-pipeline'
import {
  compileResourceEnvelope,
  type WebpackResourceBinding,
} from './webpack-resource-native'

const resourceKeys = [
  'test',
  'include',
  'exclude',
  'resource',
  'realResource',
  'compiler',
] as const
const unknownKeys = [
  'issuer',
  'issuerLayer',
  'resourceQuery',
  'resourceFragment',
  'dependency',
  'scheme',
  'assert',
  'with',
  'descriptionData',
  'mimetype',
] as const

function opaquePredicate(value: unknown): boolean {
  if (typeof value === 'function') return true
  if (value instanceof RegExp)
    return (
      value.global ||
      value.sticky ||
      Object.getPrototypeOf(value) !== RegExp.prototype
    )
  if (Array.isArray(value)) return value.some(opaquePredicate)
  if (isMdxRecord(value)) return Object.values(value).some(opaquePredicate)
  return false
}

export function createResourceEnvelope(
  binding: WebpackResourceBinding,
  path: readonly MdxRule[],
) {
  let envelope: MdxRule = { type: 'javascript/auto' }
  for (const condition of [...path].reverse()) {
    envelope = {
      ...Object.fromEntries(
        resourceKeys
          .filter(
            (name) =>
              condition[name] !== undefined &&
              !opaquePredicate(condition[name]),
          )
          .map((name) => [name, condition[name]]),
      ),
      rules: [envelope],
    }
  }
  return compileResourceEnvelope(binding, [envelope])
}

export function unknownResourceFact(
  path: readonly MdxRule[],
): string | undefined {
  return path.flatMap((item) => [
    ...unknownKeys.filter((name) => item[name] !== undefined),
    ...resourceKeys.filter((name) => opaquePredicate(item[name])),
  ])[0]
}

export function resourceRuleLabel(key: string, rule: MdxRule) {
  const defaults = key.startsWith('defaultRules.')
  const path = defaults ? key.slice('defaultRules.'.length) : key
  return {
    position: `module.${defaults ? 'defaultRules' : 'rules'}[${path.split('.')[0]}]${path.replace(/^\d+/, '').replace(/\.(rules|oneOf)\.(\d+)/g, '.$1[$2]')}`,
    test:
      rule.test instanceof RegExp
        ? String(rule.test)
        : rule.test === undefined
          ? '<inherited/all>'
          : '<native condition>',
  }
}
