import {
  createNodeModulesExcludeRegex,
  SOURCE_FILE_RE,
} from '@devup-ui/plugin-utils'
import type { Compiler, Configuration } from 'webpack'

import type { PreparedCoordinatorHandle } from './coordinator-options'
import type { MdxPipeline } from './mdx-pipeline'
import type { AppContext, AppSession } from './session'
import { createCoordinatorLoaderOptions, type RuleFields } from './turbo-rules'
import { composeWebpackMdxRules } from './webpack-coordinator-rules'

export interface WebpackPreparationBinding {
  readonly compiler: Compiler
  readonly params: Parameters<
    Compiler['hooks']['beforeCompile']['callAsync']
  >[0]
  readonly context: AppContext
  readonly session: AppSession
  readonly handle: PreparedCoordinatorHandle
  readonly pipelines: readonly MdxPipeline[]
}

export interface WebpackCoordinatorIntegration extends RuleFields {
  readonly handle: PreparedCoordinatorHandle
  /** Bind initial preparation or refresh changed inputs for each real build. */
  readonly prepare: (binding: WebpackPreparationBinding) => Promise<void>
}

/** Call only AFTER the effective user/wrapper webpack callback has returned. */
export function createWebpackCoordinatorBridge(
  integration: WebpackCoordinatorIntegration,
): (config: Configuration) => Configuration {
  const { context, session, handle, prepare } = integration
  const { shared, source } = createCoordinatorLoaderOptions(integration)
  const extraction = { loader: '@devup-ui/next-plugin/loader', options: source }
  return (config) => {
    const { rules, pipelines } = composeWebpackMdxRules(config, extraction)
    return {
      ...config,
      module: {
        ...config.module,
        rules: [
          ...rules,
          {
            test: SOURCE_FILE_RE,
            exclude: createNodeModulesExcludeRegex([...context.include]),
            enforce: 'pre',
            use: [extraction],
          },
          {
            test: /\.css$/,
            include: context.cssDir,
            enforce: 'pre',
            use: [
              { loader: '@devup-ui/next-plugin/css-loader', options: shared },
            ],
          },
        ],
      },
      plugins: [
        ...(config.plugins ?? []),
        {
          apply(compiler: Compiler) {
            compiler.hooks.beforeCompile.tapPromise(
              'DevupUINextCoordinator',
              async (params) => {
                await prepare({
                  compiler,
                  params,
                  context,
                  session,
                  handle,
                  pipelines,
                })
                await Promise.all([handle.ready, handle.prepared])
              },
            )
          },
        },
      ],
    }
  }
}
