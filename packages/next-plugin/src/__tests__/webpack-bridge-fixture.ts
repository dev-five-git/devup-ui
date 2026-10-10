import { createRequire } from 'node:module'

import { beforeAll, mock } from 'bun:test'
import type { Compiler, Configuration } from 'webpack'

import type { PreparedCoordinatorHandle } from '../coordinator-options'
import { createAppContext, createSession } from '../session'
import {
  createWebpackCoordinatorBridge,
  type WebpackCoordinatorIntegration,
} from '../webpack-coordinator'

const require = createRequire(import.meta.url)
export const webpack: typeof import('webpack') = createRequire(
  require.resolve('@types/webpack/package.json'),
)('webpack')

beforeAll(async () => {
  const instance = compiler({})
  await new Promise<void>((resolve, reject) =>
    instance.close((error) => (error ? reject(error) : resolve())),
  )
}, 30000)

export function fixture(
  overrides: Partial<WebpackCoordinatorIntegration> = {},
) {
  const context = createAppContext({}, { include: ['@included/ui'] })
  const session = createSession(context)
  const handle: PreparedCoordinatorHandle = {
    ready: Promise.resolve(),
    prepared: Promise.resolve(),
    close: mock(),
    drain: mock().mockResolvedValue(undefined),
  }
  const integration: WebpackCoordinatorIntegration = {
    context,
    session,
    handle,
    themeFiles: [context.devupFile],
    theme: { colors: { default: { primary: 'red' } } },
    prepare: mock().mockResolvedValue(undefined),
    ...overrides,
  }
  return { integration, compose: createWebpackCoordinatorBridge(integration) }
}

export function compiler(config: Configuration): Compiler {
  return webpack({ mode: 'development', ...config })
}
