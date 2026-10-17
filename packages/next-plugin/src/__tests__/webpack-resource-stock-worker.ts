import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import type * as PluginUtils from '@devup-ui/plugin-utils'

import { compilerResourceBoundary } from '../webpack-resource-compiler'
import { webpackResourceResolver } from '../webpack-resource-delivery'
import type { WebpackResourceBinding } from '../webpack-resource-native'

const [workspace, router, dev] = process.argv.slice(2)
assert.ok(workspace)
assert.ok(router === 'app' || router === 'pages')
assert.ok(dev === 'true' || dev === 'false')
assert.equal(process.versions.bun, undefined)
const scenario = { router, dev: dev === 'true' } as const
const installed = createRequire(
  join(workspace, 'packages/next-plugin/package.json'),
)
const { createModuleResolver }: typeof PluginUtils = installed(
  '@devup-ui/plugin-utils',
)
const {
  withStockClient,
}: {
  readonly withStockClient: <T>(
    scenario: { readonly router: 'app' | 'pages'; readonly dev: boolean },
    run: (binding: WebpackResourceBinding) => T | Promise<T>,
  ) => Promise<T>
} = installed(
  join(
    workspace,
    'packages/next-plugin/src/__tests__/webpack-resource-stock-fixture.cjs',
  ),
)

async function proveStockAliases() {
  // Given the installed Next config producer and its actual receiving NMF.
  const result = await withStockClient(scenario, async (binding) => {
    const original = binding.compiler.options.resolve.alias
    const entries = Object.entries(original ?? {}).map(
      ([key, value]): [string, unknown] => [
        key,
        Array.isArray(value) ? [...value] : value,
      ],
    )
    // When the helper initializes a shared resolver without filtering stock entries.
    const settings = webpackResourceResolver(binding)
    const prepared: string[] = []
    const resolver = createModuleResolver({
      cwd: binding.compiler.context,
      alias: settings.aliases,
      conditions: settings.conditions,
      prepareSource: (filename) => {
        prepared.push(filename)
        return undefined
      },
    })
    const shared = resolver(
      'private-next-instrumentation-client-user',
      join(binding.compiler.context, 'client.tsx'),
    )
    const native = await new Promise<string | false | undefined>(
      (resolve, reject) =>
        binding.params.normalModuleFactory
          .getResolver('normal')
          .resolve(
            {},
            binding.compiler.context,
            'private-next-instrumentation-client-user',
            {},
            (error, value) => (error ? reject(error) : resolve(value)),
          ),
    )
    // Then every original assertion executes against native objects in this realm.
    assert.deepEqual(Object.entries(settings.aliases), entries)
    assert.equal(binding.compiler.options.resolve.alias, original)
    assert.deepEqual(Object.entries(original ?? {}), entries)
    assert.deepEqual(shared, { ignored: true })
    assert.equal(native, false)
    assert.deepEqual(prepared, [])
    const boundary = compilerResourceBoundary(
      binding,
      join(binding.compiler.context, 'page.mdx'),
    )
    assert.equal(boundary?.rulePosition, 'webpack.externals')
    return {
      ...scenario,
      runtime: 'node',
      shared,
      native,
      prepared,
      guard: boundary.rulePosition,
    }
  })
  process.stdout.write(JSON.stringify(result))
}

void proveStockAliases()
