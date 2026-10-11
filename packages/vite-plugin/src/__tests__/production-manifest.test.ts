import { expect, it } from 'bun:test'
import { resolveConfig } from 'vite'

import {
  prepareProductionManifest,
  type ProductionOptions,
} from '../production-manifest'
import { productionFixture } from './production-fixture'

it.each([false, true])(
  'unions real condition branches and positive hoist routes for native entry records=%s',
  async (record) => {
    // Given two actual routes and different physical package condition targets.
    const fixture = await productionFixture()
    const opaque = 'virtual:manifest.js?variant=1'
    const shared = await fixture.file(
      'app/src/shared.js',
      'export const value=1;',
    )
    const a = await fixture.file(
      'app/src/a.js',
      "export {value} from './shared.js'; export {branch} from 'condition-branch';",
    )
    const b = await fixture.file(
      'app/src/b.js',
      "export {value} from './shared.js';",
    )
    await fixture.file(
      'app/node_modules/condition-branch/package.json',
      '{"name":"condition-branch","exports":{"browser":"./browser.js","node":"./node.js","default":"./node.js"}}',
    )
    const browserFile = await fixture.file(
      'app/node_modules/condition-branch/browser.js',
      'export const branch="browser";',
    )
    const nodeFile = await fixture.file(
      'app/node_modules/condition-branch/node.js',
      'export const branch="node";',
    )
    const options: ProductionOptions = {
      package: '@devup-ui/react',
      devupFile: 'devup.json',
      distDir: 'df',
      cssDir: undefined,
      extractCss: true,
      debug: false,
      include: [],
      singleCss: false,
      prefix: undefined,
      shorthands: undefined,
      sourceDirs: undefined,
      mdxExtensions: ['.mdx'],
      atomHoist: 2,
      importAliases: {},
    }
    try {
      const config = await resolveConfig(
        {
          root: fixture.root,
          configFile: false,
          logLevel: 'silent',
          environments: {
            browser: {
              consumer: 'client',
              resolve: { conditions: ['browser'] },
            },
            server: { consumer: 'server', resolve: { conditions: ['node'] } },
          },
          build: {
            lib: {
              entry: record ? { a, b, opaque } : [a, b, opaque],
              formats: ['es'],
            },
          },
        },
        'build',
      )
      // When the real manifest closes independent context graphs before allocation.
      const manifest = await prepareProductionManifest(
        config,
        options,
        () => {},
      )
      // Then entry forms select distinct physical branches and retain a two-route hoist plan.
      const browser = manifest.contexts.find(
        (context) => context.context === 'browser',
      )
      const server = manifest.contexts.find(
        (context) => context.context === 'server',
      )
      expect(browser?.scan).toContain(browserFile)
      expect(browser?.scan).not.toContain(nodeFile)
      expect(server?.scan).toContain(nodeFile)
      expect(server?.scan).not.toContain(browserFile)
      expect(browser?.threshold).toBe(2)
      expect(browser?.scan).toContain('virtual:manifest.js')
      expect(server?.scan).toContain('virtual:manifest.js')
      expect(browser?.routes[shared]).toHaveLength(2)
    } finally {
      await fixture.close()
    }
  },
)

it('reserves all configured contexts while keeping scan authority separate from learned IDs', async () => {
  // Given an ordinary entry, plain source, selected Markdown and resolved independent environments.
  const fixture = await productionFixture()
  const entry = await fixture.file('app/src/main.js', 'export const value=1;')
  const plain = await fixture.file(
    'app/src/plain.test.ts',
    'export const plain=1;',
  )
  const markdown = await fixture.file(
    'app/src/docs.mdx',
    '# Reserved documentation',
  )
  const options: ProductionOptions = {
    package: '@devup-ui/react',
    devupFile: 'devup.json',
    distDir: 'df',
    cssDir: undefined,
    extractCss: true,
    debug: false,
    include: [],
    singleCss: false,
    prefix: undefined,
    shorthands: undefined,
    sourceDirs: undefined,
    mdxExtensions: ['.mdx'],
    atomHoist: undefined,
    importAliases: {},
  }
  try {
    const config = await resolveConfig(
      {
        root: fixture.root,
        configFile: false,
        logLevel: 'silent',
        environments: {
          browser: {
            consumer: 'client',
            resolve: { conditions: ['browser', 'production'] },
          },
          server: {
            consumer: 'server',
            resolve: { conditions: ['node', 'react-server'] },
          },
        },
        build: { lib: { entry, formats: ['es'] } },
      },
      'build',
    )
    // When allocation-inert reservation closes every final context.
    const manifest = await prepareProductionManifest(config, options, () => {})
    // Then plain/test/Markdown physical IDs are reserved in each named context with independent conditions.
    const browser = manifest.contexts.find(
      (context) => context.context === 'browser',
    )
    const server = manifest.contexts.find(
      (context) => context.context === 'server',
    )
    expect(browser?.scan).toEqual(
      expect.arrayContaining([entry, plain, markdown]),
    )
    expect(server?.scan).toEqual(
      expect.arrayContaining([entry, plain, markdown]),
    )
    expect(browser?.conditions).toContain('browser')
    expect(server?.conditions).toContain('react-server')
    expect(browser?.learned).toEqual([])
    expect(browser?.canonical).toEqual({})
    expect(browser?.routes).toEqual({})
  } finally {
    await fixture.close()
  }
})
