import { describe, expect, it } from 'bun:test'
import type { Configuration } from 'webpack'

import { createTurboRules } from '../turbo-rules'
import { compiler, fixture } from './webpack-bridge-fixture'

describe('Next webpack coordinator bridge', () => {
  it('preserves effective callback results and conditions when composing loaders', () => {
    // Given
    const { compose, integration } = fixture()
    const options = { jsx: true, remarkPlugins: [() => {}] }
    const unrelated = { test: /\.png$/, type: 'asset/resource' }
    const effective: Configuration = {
      name: 'returned-by-wrapper',
      resolve: { alias: { provider: '/app/provider.js' } },
      module: {
        parser: { javascript: { dynamicImportMode: 'lazy' } },
        rules: [
          false,
          '...',
          {
            issuer: /page/,
            rules: [
              {
                oneOf: [
                  unrelated,
                  {
                    test: /\.mdx$/,
                    include: '/app',
                    use: [
                      'next-flight-loader',
                      'next-swc-loader',
                      {
                        loader: '@next/mdx/mdx-js-loader',
                        options,
                      },
                    ],
                  },
                ],
              },
            ],
          },
        ],
      },
      plugins: [{ apply() {} }],
    }
    // When
    const result = compose(effective)
    // Then
    expect(result.name).toBe(effective.name)
    expect(result.resolve).toBe(effective.resolve)
    expect(result.module?.parser).toBe(effective.module?.parser)
    expect(result.plugins?.[0]).toBe(effective.plugins?.[0])
    expect(result.module?.rules?.slice(0, 2)).toEqual([false, '...'])
    expect(result.module?.rules?.[2]).toEqual({
      issuer: /page/,
      rules: [
        {
          oneOf: [
            unrelated,
            {
              test: /\.mdx$/,
              include: '/app',
              use: [
                'next-flight-loader',
                'next-swc-loader',
                {
                  loader: '@devup-ui/next-plugin/loader',
                  options: expect.objectContaining({
                    coordinatorPortFile: integration.session.endpointFile,
                    coordinatorIdentity: integration.session.identity,
                  }),
                },
                { loader: '@next/mdx/mdx-js-loader', options },
              ],
            },
          ],
        },
      ],
    })
    expect(effective.module?.rules).toHaveLength(3)
  })

  it('uses captured Turbo options for ordinary sources and generated CSS', () => {
    // Given
    const { compose, integration } = fixture()
    const turbo = createTurboRules(integration)
    // When
    const rules = compose({}).module?.rules
    // Then
    expect(rules).toEqual([
      {
        test: expect.any(RegExp),
        exclude: expect.any(RegExp),
        enforce: 'pre',
        use: [
          {
            loader: '@devup-ui/next-plugin/loader',
            options: expect.objectContaining({
              projectRoot: integration.context.root,
              requestTimeoutMs: 120000,
              coordinatorPortFile: integration.session.endpointFile,
              importAliases: integration.context.importAliases,
            }),
          },
        ],
      },
      {
        test: /\.css$/,
        include: integration.context.cssDir,
        enforce: 'pre',
        use: [
          {
            loader: '@devup-ui/next-plugin/css-loader',
            options: expect.objectContaining({
              coordinatorPortFile: integration.session.endpointFile,
            }),
          },
        ],
      },
    ])
    expect(Object.values(turbo)).toHaveLength(2)
  })

  it('awaits binding plus both readiness gates before compilation work', async () => {
    // Given
    const binding = Promise.withResolvers<void>()
    const transport = Promise.withResolvers<void>()
    const prepared = Promise.withResolvers<void>()
    const entered = Promise.withResolvers<void>()
    const { compose, integration } = fixture({
      handle: {
        ready: transport.promise,
        prepared: prepared.promise,
        close() {},
        async drain() {},
      },
      async prepare(request) {
        entered.resolve()
        expect(request.compiler.context).toBe(integration.context.root)
        expect(request.context).toBe(integration.context)
        expect(request.session).toBe(integration.session)
        expect(request.handle).toBe(integration.handle)
        expect(request.params.normalModuleFactory).toBeDefined()
        await binding.promise
      },
    })
    const instance = compiler(compose({ context: integration.context.root }))
    let completed = false
    // When
    const work = instance.hooks.beforeCompile
      .promise(instance.newCompilationParams())
      .then(() => {
        completed = true
      })
    await entered.promise
    expect(completed).toBe(false)
    binding.resolve()
    prepared.resolve()
    await Promise.resolve()
    expect(completed).toBe(false)
    transport.resolve()
    await work
    // Then
    expect(completed).toBe(true)
  })

  it('retains the original located failure when preparation rejects', async () => {
    // Given
    const cause = new Error('/app/page.mdx:7:3: required compilation failed')
    const { compose } = fixture({
      prepare: async () => {
        throw cause
      },
    })
    const instance = compiler(compose({}))
    // When / Then
    await expect(
      instance.hooks.beforeCompile.promise(instance.newCompilationParams()),
    ).rejects.toBe(cause)
  })

  it('refreshes each server and client build without releasing their shared owner', async () => {
    // Given
    const requests: string[] = []
    const { compose, integration } = fixture({
      prepare: async ({ compiler, handle }) => {
        expect(handle).toBe(integration.handle)
        requests.push(compiler.name ?? '')
      },
    })
    const server = compiler(compose({ name: 'server' }))
    const client = compiler(compose({ name: 'client' }))
    // When
    await server.hooks.beforeCompile.promise(server.newCompilationParams())
    await client.hooks.beforeCompile.promise(client.newCompilationParams())
    await server.hooks.beforeCompile.promise(server.newCompilationParams())
    await new Promise<void>((resolve, reject) =>
      server.close((error) => (error ? reject(error) : resolve())),
    )
    await client.hooks.beforeCompile.promise(client.newCompilationParams())
    // Then
    expect(requests).toEqual(['server', 'client', 'server', 'client'])
    expect(integration.handle.close).not.toHaveBeenCalled()
    expect(integration.handle.drain).not.toHaveBeenCalled()
  })
})
