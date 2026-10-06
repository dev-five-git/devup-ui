import type {
  ModuleAliasDescriptor,
  ModuleAliasOptions,
} from '@devup-ui/plugin-utils'

import { isMdxRecord } from './mdx-pipeline'
import type { WebpackResourceBinding } from './webpack-resource-native'

export function webpackResourceDelivery(binding: WebpackResourceBinding) {
  const options = binding.compiler.options
  const devtool = options.devtool
  const full =
    typeof devtool === 'string' &&
    devtool.includes('source-map') &&
    (!devtool.includes('cheap') || devtool.includes('module'))
  const loader: unknown = options.loader
  const sourceMap =
    isMdxRecord(loader) && loader.sourceMap !== undefined
      ? loader.sourceMap
      : full
  const mode =
    isMdxRecord(loader) && loader.mode !== undefined
      ? loader.mode
      : options.mode || 'production'
  if (
    typeof sourceMap !== 'boolean' ||
    (mode !== 'production' && mode !== 'development' && mode !== 'none')
  )
    throw new TypeError(
      'effective native loader map/mode override is unsupported',
    )
  return Object.freeze({ sourceMap, mode })
}

function immutableAliasTarget(target: ModuleAliasDescriptor['alias']) {
  return typeof target === 'object' ? Object.freeze([...target]) : target
}

function immutableModuleAliases(alias: ModuleAliasOptions): ModuleAliasOptions {
  if (alias === false) return false
  return Array.isArray(alias)
    ? Object.freeze(
        alias.map((item: ModuleAliasDescriptor) =>
          Object.freeze({ ...item, alias: immutableAliasTarget(item.alias) }),
        ),
      )
    : Object.freeze(
        Object.fromEntries<ModuleAliasDescriptor['alias']>(
          Object.entries(alias).map(([key, value]) => [
            key,
            immutableAliasTarget(value),
          ]),
        ),
      )
}

export function webpackResourceResolver(binding: WebpackResourceBinding): {
  readonly aliases: ModuleAliasOptions
  readonly conditions: readonly string[]
} {
  const alias = binding.effectiveConfiguration.resolve?.alias ?? {}
  const aliases = immutableModuleAliases(alias)
  const resolver = binding.compiler.resolverFactory.get('normal')
  const conditions: unknown = resolver.options.conditionNames
  if (
    !(conditions instanceof Set) ||
    ![...conditions].every((value: unknown) => typeof value === 'string')
  )
    throw new TypeError('effective native resolver conditions are unavailable')
  const values: string[] = []
  for (const value of conditions)
    if (typeof value === 'string') values.push(value)
  return Object.freeze({
    aliases,
    conditions: Object.freeze(values),
  })
}
