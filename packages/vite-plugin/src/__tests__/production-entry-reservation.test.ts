import { symlink } from 'node:fs/promises'

import {
  createNonphysicalIdStore,
  resolveSourceDirs,
} from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'
import { resolveConfig, type UserConfig } from 'vite'

import {
  prepareProductionManifest,
  type ProductionOptions,
} from '../production-manifest'
import { productionRootSources } from '../production-root-sources'
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
  'reserves roots and input closure while preserving legacy inventory when input=%s',
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
      // Then ordinary HTML retains 919 inventory, while scripts also reserve configured roots.
      const legacy = form === 'html' || form === 'html-mixed'
      const expected =
        form === 'virtual'
          ? [unused]
          : [
              entry,
              view,
              ...(form === 'mdx' ? [mdx] : []),
              unused,
              ...(legacy ? [tooling] : []),
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
      expect(context?.scan).toContain(unused)
      if (form !== 'virtual')
        expect(observed).toContain(
          entry.replaceAll('/', process.platform === 'win32' ? '\\' : '/'),
        )
    } finally {
      await fixture.close()
    }
  },
)

it('censuses explicit roots and includes without test, output or automatic package inventories', async () => {
  // Given default roots, explicit overrides, linked siblings and an included package.
  const fixture = await productionFixture()
  const source = await fixture.file('app/src/view.js', 'export {};')
  const app = await fixture.file('app/app/page.js', 'export {};')
  const configured = await fixture.file('app/custom/view.js', 'export {};')
  const included = await fixture.file(
    'app/node_modules/included/src/view.js',
    'export {};',
  )
  await fixture.file(
    'app/node_modules/included/package.json',
    '{"name":"included"}',
  )
  for (const name of '__tests__ tests test __fixtures__ fixtures dist build out coverage .next tooling scripts generated'.split(
    ' ',
  ))
    await fixture.file(`app/src/${name}/view.js`, 'export {};')
  for (const name of 'view.test.tsx view.spec.js types.d.ts types.d.mts types.d.cts'.split(
    ' ',
  ))
    await fixture.file(`app/src/${name}`, 'export {};')
  await fixture.file('app/node_modules/included/dist/unused.js', 'export {};')
  await fixture.file(
    'app/node_modules/included/fixtures/unused.js',
    'export {};',
  )
  await fixture.file(
    'app/node_modules/@devup-ui/tooling/package.json',
    '{"main":"index.js"}',
  )
  await fixture.file(
    'app/node_modules/@devup-ui/tooling/index.js',
    'export {};',
  )
  const outside = await fixture.file('outside/view.js', 'export {};')
  for (const name of ['first', 'second'])
    await symlink(
      `${fixture.directory}/outside`,
      `${fixture.root}/src/${name}`,
      'junction',
    )
  await symlink(
    `${fixture.root}/src`,
    `${fixture.directory}/outside/cycle`,
    'junction',
  )
  await symlink(
    `${fixture.root}/src/fixtures`,
    `${fixture.root}/src/blocked`,
    'junction',
  )
  try {
    // When the same private census uses default roots or explicit sourceDirs.
    const options = {
      cwd: fixture.root,
      exclude: [`${fixture.root}/src/generated`],
      includeMdx: ['.mdx'],
    }
    const defaults = productionRootSources(
      { ...options, roots: resolveSourceDirs(fixture.root) },
      ['included'],
    )
    const override = productionRootSources(
      { ...options, roots: resolveSourceDirs(fixture.root, ['custom']) },
      [],
    )
    // Then lexical aliases survive cycles; excluded census sources never join either set.
    const paths = defaults.map(({ path }) => path.replaceAll('\\', '/')).sort()
    expect(paths).toEqual(
      [
        source,
        app,
        included,
        `${fixture.root}/src/first/view.js`,
        `${fixture.root}/src/second/view.js`,
      ].sort(),
    )
    expect(
      defaults.filter(
        ({ realPath }) => realPath.replaceAll('\\', '/') === outside,
      ),
    ).toHaveLength(2)
    expect(override.map(({ path }) => path.replaceAll('\\', '/'))).toEqual([
      configured,
    ])
  } finally {
    await fixture.close()
  }
})
