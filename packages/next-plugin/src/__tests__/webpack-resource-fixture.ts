import { createRequire } from 'node:module'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { Compiler, Configuration, RuleSetRule } from 'webpack'

import { composeWebpackMdxRules } from '../webpack-coordinator-rules'
import type { WebpackResourceBinding } from '../webpack-resource-native'
import { createWebpackResourceSelector } from '../webpack-resource-selector'

const workspace = fileURLToPath(new URL('../../../../', import.meta.url))
const installed = createRequire(join(workspace, 'apps/landing/package.json'))
const bundle: { readonly webpack: typeof import('webpack') } = installed(
  'next/dist/compiled/webpack/webpack',
)
export const mdxLoader = installed.resolve('@next/mdx/mdx-js-loader')
export const extraction = '@devup-ui/next-plugin/loader'
export const source = join(workspace, 'w21e-resource-page.mdx')

export function mdxRule(
  options: Readonly<Record<string, unknown>> = {},
): RuleSetRule {
  return { test: /\.mdx$/, use: [{ loader: mdxLoader, options }] }
}

export async function withSelector<T>(
  config: Configuration,
  run: (
    selector: Awaited<ReturnType<typeof createWebpackResourceSelector>>,
    compiler: Compiler,
    binding: WebpackResourceBinding,
  ) => Promise<T> | T,
): Promise<T> {
  const composed = composeWebpackMdxRules(config, { loader: extraction })
  const effective: Configuration = {
    ...config,
    mode: config.mode ?? 'production',
    optimization: { minimize: false },
    entry: {},
    module: { ...config.module, rules: composed.rules },
  }
  const compiler = bundle.webpack(effective)
  let result: T | undefined
  compiler.hooks.beforeCompile.tapPromise(
    'W21eResourceFixture',
    async (params) => {
      const binding = {
        compiler,
        params,
        effectiveConfiguration: compiler.options,
        pipelines: composed.pipelines,
      }
      const selector = await createWebpackResourceSelector({
        binding,
        configFile: join(workspace, 'next.config.mjs'),
        owner: {},
        generation: {},
      })
      result = await run(selector, compiler, binding)
    },
  )
  try {
    await new Promise<void>((resolve, reject) =>
      compiler.compile((error) => (error ? reject(error) : resolve())),
    )
    if (result === undefined)
      throw new TypeError('fixture callback must return a value')
    return result
  } finally {
    await new Promise<void>((resolve, reject) =>
      compiler.close((error) => (error ? reject(error) : resolve())),
    )
  }
}
