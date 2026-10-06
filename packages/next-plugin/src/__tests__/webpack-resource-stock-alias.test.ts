import { createRequire } from 'node:module'
import { join } from 'node:path'

import { createModuleResolver } from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'

import { compilerResourceBoundary } from '../webpack-resource-compiler'
import { webpackResourceResolver } from '../webpack-resource-delivery'
import type { WebpackResourceBinding } from '../webpack-resource-native'

const {
  withStockClient,
}: {
  readonly withStockClient: <T>(
    scenario: { readonly router: 'app' | 'pages'; readonly dev: boolean },
    run: (binding: WebpackResourceBinding) => T | Promise<T>,
  ) => Promise<T>
} = createRequire(import.meta.url)('./webpack-resource-stock-fixture.cjs')

it.each([
  { router: 'app', dev: true },
  { router: 'app', dev: false },
  { router: 'pages', dev: true },
  { router: 'pages', dev: false },
] as const)(
  'represents unchanged stock client aliases and ignores instrumentation fallback in %j',
  async (scenario) => {
    // Given the installed Next config producer and its actual receiving NMF.
    await withStockClient(scenario, async (binding) => {
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
      const result = resolver(
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
      // Then aliases are lossless and ignore has no path/code/preparation authority.
      expect(entries).toEqual(Object.entries(settings.aliases))
      expect(binding.compiler.options.resolve.alias).toBe(original)
      expect(Object.entries(original ?? {})).toEqual(entries)
      expect(result).toEqual({ ignored: true })
      expect(native).toBe(false)
      expect(prepared).toEqual([])
      expect(
        compilerResourceBoundary(
          binding,
          join(binding.compiler.context, 'page.mdx'),
        ),
      ).toMatchObject({ rulePosition: 'webpack.externals' })
      return true
    })
  },
)
