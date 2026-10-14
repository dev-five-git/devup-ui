import { mergeImportAliases } from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'
import { resolveConfig } from 'vite'

import { NonphysicalModules } from '../nonphysical-modules'
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
      '/* @devup-ui/react */ export const value=1;',
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
    for (const branch of ['browser', 'node']) {
      await fixture.file(
        `app/node_modules/condition-branch/${branch}.js`,
        `export {branch} from './${branch}/node_modules/condition-target/index.js';`,
      )
      await fixture.file(
        `app/node_modules/condition-branch/${branch}/node_modules/condition-target/package.json`,
        '{"name":"condition-target","main":"index.js"}',
      )
    }
    const browserFile = await fixture.file(
      'app/node_modules/condition-branch/browser/node_modules/condition-target/index.js',
      '/* @devup-ui/react */ export const branch="browser";',
    )
    const nodeFile = await fixture.file(
      'app/node_modules/condition-branch/node/node_modules/condition-target/index.js',
      '/* @devup-ui/react */ export const branch="node";',
    )
    const options: ProductionOptions = {
      package: '@devup-ui/react',
      devupFile: 'devup.json',
      distDir: 'df',
      cssDir: undefined,
      extractCss: true,
      debug: false,
      include: ['condition-target'],
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
  const accepted: string[] = []
  const rejected = [entry, plain, markdown]
  for (const name of ['included', 'excluded'])
    await fixture.file(
      `app/node_modules/${name}/package.json`,
      JSON.stringify({ name, main: 'index.js' }),
    )
  await fixture.file(
    'app/src/imports.js',
    "import 'included'; import 'excluded';",
  )
  for (const [name, needle, included] of [
    ['styled.js', 'custom-target', true],
    ['stylex.js', '@stylexjs/stylex', true],
    ['alias.js', 'custom-alias', true],
    ['disabled.js', '@emotion/react', false],
    ['default.js', '@devup-ui/react', false],
    ['types.d.ts', 'custom-target', false],
    ['types.d.mts', 'custom-target', false],
    ['types.d.cts', 'custom-target', false],
    ['../node_modules/included/index.js', 'custom-target', true],
    ['../node_modules/excluded/index.js', 'custom-target', false],
  ] as const) {
    const path = await fixture.file(
      `app/src/${name}`,
      `/* ${needle} */ export {};`,
    )
    ;(included ? accepted : rejected).push(path)
  }
  const options: ProductionOptions = {
    package: 'custom-target',
    devupFile: 'devup.json',
    distDir: 'df',
    cssDir: undefined,
    extractCss: true,
    debug: false,
    include: ['included'],
    singleCss: false,
    prefix: undefined,
    shorthands: undefined,
    sourceDirs: undefined,
    mdxExtensions: ['.mdx'],
    atomHoist: undefined,
    importAliases: mergeImportAliases({
      '@emotion/react': false,
      'custom-alias': true,
    }),
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
    // Then own raw source admits only eligible needles in every configured context.
    const browser = manifest.contexts.find(
      (context) => context.context === 'browser',
    )
    const server = manifest.contexts.find(
      (context) => context.context === 'server',
    )
    for (const context of manifest.contexts) {
      for (const path of accepted) expect(context.scan).toContain(path)
      for (const path of rejected) expect(context.scan).not.toContain(path)
    }
    expect(browser?.conditions).toContain('browser')
    expect(server?.conditions).toContain('react-server')
    expect(browser?.learned).toEqual([])
    expect(browser?.canonical).toEqual({})
    expect(browser?.routes).toEqual({})
  } finally {
    await fixture.close()
  }
})

it.each([false, true, undefined])(
  'joins every source occurrence before learning and resets interval proof when later relevance=%s',
  async (relevant) => {
    // Given two native query occurrences sharing an existing normalized extraction ID.
    const modules = new NonphysicalModules([])
    const id = 'virtual:proof.js'
    let reads = 0
    const source = {
      path: id,
      get code() {
        reads += 1
        return 'export {};'
      },
    }
    const graph = {
      getModuleIds: () => [][Symbol.iterator](),
      getModuleInfo: () => null,
    }
    modules.direct(`${id}?first`, id, source, () => false)
    modules.resolved(
      `${id}?first`,
      source,
      relevant === undefined ? undefined : () => relevant,
    )
    // When unchanged completion rules finish the observed interval.
    const completed = await modules.finish(graph, [])
    // Then relevant or unknown evidence wins without discarding O or ready attachments.
    expect(completed.ids.includes(id)).toBe(relevant !== false)
    expect(modules.observations()).toContain(id)
    expect(modules.source(id)).toBeDefined()
    expect(reads).toBe(1)
    modules.start()
    modules.direct(`${id}?next`, id, source, () => false)
    expect((await modules.finish(graph, [])).ids).not.toContain(id)
    expect(reads).toBe(1)
  },
)
