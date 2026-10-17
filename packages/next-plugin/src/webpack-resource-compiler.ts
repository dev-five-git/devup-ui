import { isAbsolute } from 'node:path'

import type { WebpackResourceBoundary } from './webpack-resource-error'
import type { WebpackResourceBinding } from './webpack-resource-native'

export function compilerResourceBoundary(
  binding: WebpackResourceBinding,
  filename: string,
): WebpackResourceBoundary | undefined {
  if (!isAbsolute(filename) || /[!?#]/.test(filename))
    return Object.freeze({
      filename,
      rulePosition: 'module.rules',
      test: '<disk resource>',
      unknownFact: 'plain disk resource',
      reason:
        'inline requests, matchResource, schemes and query/fragment substitutions require native origin facts; use an absolute unsubstituted disk resource',
    })
  const externals = binding.effectiveConfiguration.externals
  if (
    externals !== undefined &&
    (!Array.isArray(externals) || externals.length > 0)
  )
    return Object.freeze({
      filename,
      rulePosition: 'webpack.externals',
      test: '<factory callout>',
      unknownFact: 'native factory substitution',
      reason:
        'beforeCompile RuleSet does not prove the effects of external-module factorization installed for the later compilation',
    })
  const plugins = binding.effectiveConfiguration.plugins ?? []
  if (plugins.length > 0)
    return Object.freeze({
      filename,
      rulePosition: 'webpack.plugins[0]',
      test: '<compilation callout>',
      unknownFact: 'pitch/source-reader/factory substitution',
      reason:
        'an opaque plugin can install module/source-reader hooks after beforeCompile; effective configuration carries no source-proven original Next callout identity',
    })
  if (binding.effectiveConfiguration.resolveLoader?.plugins?.length)
    return Object.freeze({
      filename,
      rulePosition: 'webpack.resolveLoader.plugins',
      test: '<resolver callout>',
      unknownFact: 'loader resolver context',
      reason:
        'a custom loader resolver can use missing issuer/dependency facts',
    })
  return
}
