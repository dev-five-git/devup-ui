import { mergeImportAliases } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { resolveConfig } from 'vite'

import { DevupUI } from '../plugin'
import {
  prepareProductionManifest,
  type ProductionOptions,
} from '../production-manifest'
import { productionFixture } from './production-fixture'

it('does not reserve diagnostic-only companion imports in the production manifest', async () => {
  // Given real companion value imports, without any ordinary extraction needle.
  const fixture = await productionFixture()
  const entry = await fixture.file('app/src/main.js', 'export {};')
  const companions: string[] = []
  for (const [name, binding] of [
    ['recipes', 'recipe'],
    ['sprinkles', 'createSprinkles'],
  ] as const)
    companions.push(
      await fixture.file(
        `app/src/${name}.ts`,
        `import {${binding}} from '@vanilla-extract/${name}'; export const value=${binding}({});`,
      ),
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
    importAliases: mergeImportAliases(),
  }
  try {
    const config = await resolveConfig(
      {
        root: fixture.root,
        configFile: false,
        logLevel: 'silent',
        build: { lib: { entry, formats: ['es'] } },
      },
      'build',
    )
    // When real discovery prepares numbering plans with the default CSS alias enabled.
    const manifest = await prepareProductionManifest(config, options, () => {})
    // Then companion diagnostics have no allocation, scan or learned-ID authority.
    for (const context of manifest.contexts) {
      for (const companion of companions) {
        expect(context.files.map((file) => file.id)).not.toContain(companion)
        expect(context.scan).not.toContain(companion)
        expect(context.nonphysical).not.toContain(companion)
      }
      expect(context.learned).toEqual([])
    }
  } finally {
    await fixture.close()
  }
})

it.each([
  ['@vanilla-extract/recipes', 'recipe'],
  ['@vanilla-extract/sprinkles', 'createSprinkles'],
] as const)(
  'reports a located production companion diagnostic for %s without a styling needle',
  async (packageName, binding) => {
    // Given an ordinary selected TS value import, external to the native bundler.
    const fixture = await productionFixture()
    const source = await fixture.file(
      'app/src/companion.ts',
      `import {${binding}} from '${packageName}'; export const value=${binding}({});`,
    )
    await fixture.file(
      'app/src/main.js',
      "export {value} from './companion.ts';",
    )
    try {
      // When the real production transform runs with default compatibility aliases.
      const result = fixture.run(false, [], {
        build: {
          write: false,
          lib: { entry: `${fixture.root}/src/main.js`, formats: ['es'] },
          rollupOptions: { external: [packageName] },
        },
      })
      // Then native checking, rather than bundler resolution, supplies the location.
      await expect(result).rejects.toThrow(`${source}:1:1:`)
      await expect(result).rejects.toThrow(`\`${packageName}\` generates CSS`)
    } finally {
      await fixture.close()
    }
  },
  60000,
)

it.each([
  ['disabled', 'recipes', 'recipe'],
  ['disabled', 'sprinkles', 'createSprinkles'],
  ['type-only', 'recipes', 'recipe'],
  ['type-only', 'sprinkles', 'createSprinkles'],
  ['mention-only', 'recipes', 'recipe'],
  ['mention-only', 'sprinkles', 'createSprinkles'],
] as const)(
  'keeps diagnostic-only companion %s %s source out of reservation and learning',
  async (kind, name, binding) => {
    // Given a harmless companion source and a genuinely styled sibling.
    const fixture = await productionFixture()
    const packageName = `@vanilla-extract/${name}`
    const sources = {
      disabled: `import {${binding}} from '${packageName}'; export const value=${binding}({});`,
      'type-only': `import type {${binding}} from '${packageName}'; export const value=1;`,
      'mention-only': `/* ${packageName} */ export const value='${packageName}';`,
    }
    const companion = await fixture.file('app/src/companion.ts', sources[kind])
    await fixture.file(
      'app/src/main.js',
      "export {value} from './companion.ts'; export {cls} from './style.js';",
    )
    await fixture.file(
      'app/src/style.js',
      "import {css} from '@devup-ui/react'; export const cls=css({color:'red'});",
    )
    try {
      // When the native build completes with a disabled alias or non-value mention.
      await fixture.run(false, [], {
        plugins: [
          DevupUI({
            importAliases: {
              '@vanilla-extract/css': kind !== 'disabled',
            },
          }),
        ],
        build: {
          write: false,
          lib: { entry: `${fixture.root}/src/main.js`, formats: ['es'] },
          rollupOptions: { external: [packageName] },
        },
      })
      // Then diagnostic eligibility has not become a numbering or persisted-ID needle.
      const files: unknown = JSON.parse(wasm.exportFileMap())
      expect(files).not.toEqual(
        expect.objectContaining({ [companion]: expect.anything() }),
      )
      expect(await fixture.learned()).not.toContain(companion)
    } finally {
      await fixture.close()
    }
  },
  60000,
)
