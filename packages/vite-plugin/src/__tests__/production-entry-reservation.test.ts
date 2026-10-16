import { createNonphysicalIdStore } from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'
import { resolveConfig, type UserConfig } from 'vite'

import {
  prepareProductionManifest,
  type ProductionOptions,
} from '../production-manifest'
import { productionFixture } from './production-fixture'

it.each([
  'script',
  'lib-array',
  'lib-object',
  'lib-native',
  'ssr',
  'environment',
  'virtual',
  'mdx',
  'html',
  'html-mixed',
] as const)(
  'reserves input closure while preserving legacy inventory when input=%s',
  async (form) => {
    // Given exact inputs outside sourceDirs and an unrelated configured search file.
    const fixture = await productionFixture()
    const entry = await fixture.file(
      'app/entry/main.js',
      "/* @devup-ui/react */ import './view.js';",
    )
    const view = await fixture.file(
      'app/entry/view.js',
      '/* @devup-ui/react */ export {};',
    )
    const unused = await fixture.file(
      'app/search/unused.js',
      '/* @devup-ui/react */ export {};',
    )
    const mdx = await fixture.file(
      'app/entry/page.mdx',
      '# No raw extraction needle',
    )
    await fixture.file(
      'app/node_modules/@devup-ui/tooling/package.json',
      '{"main":"index.js"}',
    )
    const tooling = await fixture.file(
      'app/node_modules/@devup-ui/tooling/index.js',
      '/* @devup-ui/react */ export {};',
    )
    const html = await fixture.file(
      'app/index.html',
      '<script type="module" src="/entry/main.js"></script>',
    )
    const opaque = 'virtual:framework-entry?variant=1'
    const options: ProductionOptions = {
      package: '@devup-ui/react',
      devupFile: 'devup.json',
      distDir: 'df',
      cssDir: undefined,
      sourceDirs: ['search'],
      mdxExtensions: ['.mdx'],
      prefix: undefined,
      shorthands: undefined,
      atomHoist: undefined,
      importAliases: {},
      include: [],
      singleCss: false,
      extractCss: true,
      debug: false,
    }
    let input: UserConfig = {}
    switch (form) {
      case 'script':
        input = { build: { rollupOptions: { input: entry } } }
        break
      case 'lib-array':
        input = { build: { lib: { entry: [entry], formats: ['es'] } } }
        break
      case 'lib-object':
        input = { build: { lib: { entry: { main: entry }, formats: ['es'] } } }
        break
      case 'lib-native':
        input = {
          build: { lib: { entry: unused }, rolldownOptions: { input: entry } },
        }
        break
      case 'ssr':
        input = { build: { ssr: entry, rollupOptions: { input: unused } } }
        break
      case 'environment':
        input = { environments: { client: { input: entry } } }
        break
      case 'virtual':
        input = { build: { rollupOptions: { input: opaque } } }
        break
      case 'mdx':
        await fixture.file(
          'app/entry/main.js',
          "/* @devup-ui/react */ import './view.js'; import './page.mdx';",
        )
        input = { build: { rollupOptions: { input: entry } } }
        break
      case 'html':
        input = {}
        break
      case 'html-mixed':
        input = { build: { rollupOptions: { input: { html, opaque } } } }
        break
      default:
        form satisfies never
    }
    const store = createNonphysicalIdStore({
      integration: 'Vite',
      resolvedRoot: fixture.root,
      contextKey: 'client',
      distDir: `${fixture.root}/df`,
    })
    await store.rewrite(['virtual:learned.js'], [])
    try {
      const config = await resolveConfig(
        { root: fixture.root, configFile: false, logLevel: 'silent', ...input },
        'build',
      )
      const observed: string[] = []
      // When allocation-inert preparation discovers sources through the existing scanner.
      const manifest = await prepareProductionManifest(
        config,
        options,
        (inputs) => observed.push(...inputs.fileDependencies),
      )
      // Then ordinary HTML retains 919 inventory, while scripts reserve only reached candidates.
      const legacy = form === 'html' || form === 'html-mixed'
      const expected =
        form === 'virtual'
          ? []
          : [
              entry,
              view,
              ...(form === 'mdx' ? [mdx] : []),
              ...(legacy ? [unused, tooling] : []),
            ]
      const context = manifest.contexts.find(
        ({ context }) => context === 'client',
      )
      expect(context?.files.map(({ id }) => id).sort()).toEqual(expected.sort())
      expect(context?.nonphysical).toEqual([
        ...(form === 'html-mixed' ? ['virtual:framework-entry'] : []),
        'virtual:learned.js',
      ])
      expect(context?.scan).not.toContain('virtual:learned.js')
      if (!legacy) expect(context?.scan).not.toContain(unused)
      if (form !== 'virtual')
        expect(observed).toContain(
          entry.replaceAll('/', process.platform === 'win32' ? '\\' : '/'),
        )
    } finally {
      await fixture.close()
    }
  },
)
