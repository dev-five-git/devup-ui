import type { ModuleAliases } from '@devup-ui/plugin-utils'

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

export function webpackResourceResolver(binding: WebpackResourceBinding): {
  readonly aliases: ModuleAliases
  readonly conditions: readonly string[]
} {
  const aliases: Record<string, string | readonly string[]> = {}
  const alias = binding.effectiveConfiguration.resolve?.alias ?? {}
  if (Array.isArray(alias)) {
    for (const item of alias) {
      const key = `${item.name}${item.onlyModule ? '$' : ''}`
      if (Object.hasOwn(aliases, key) || item.alias === false)
        throw new TypeError(
          'native alias descriptor order cannot be represented losslessly',
        )
      Object.defineProperty(aliases, key, {
        value: Object.freeze(
          typeof item.alias === 'string' ? [item.alias] : [...item.alias],
        ),
        enumerable: true,
      })
    }
  } else {
    for (const [key, value] of Object.entries(alias)) {
      if (value === false)
        throw new TypeError(
          'disabled native aliases cannot be represented losslessly',
        )
      Object.defineProperty(aliases, key, {
        value: typeof value === 'string' ? value : Object.freeze([...value]),
        enumerable: true,
      })
    }
  }
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
    aliases: Object.freeze(aliases),
    conditions: Object.freeze(values),
  })
}
