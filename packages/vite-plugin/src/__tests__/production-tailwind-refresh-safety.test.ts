import { beginBuild, MixedBuildIntegrationError } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { resolveConfig } from 'vite'

import { productionEngine } from '../production-engine'
import { ProductionActivation } from '../production-numbering'
import { productionFixture } from './production-fixture'

const contexts = ['client', 'server'] as const
const themeCode =
  "import {Box} from '@devup-ui/react'; export const view=<Box className='bg-brand'/>;"

async function withOwner(
  action: (
    activation: ProductionActivation,
    fixture: Awaited<ReturnType<typeof productionFixture>>,
  ) => Promise<void>,
) {
  const fixture = await productionFixture()
  const activation = new ProductionActivation(
    {
      package: '@devup-ui/react',
      devupFile: 'config/devup.json',
      distDir: 'df',
      cssDir: undefined,
      extractCss: true,
      debug: false,
      include: [],
      singleCss: true,
      prefix: undefined,
      shorthands: undefined,
      sourceDirs: undefined,
      mdxExtensions: ['.mdx'],
      atomHoist: undefined,
      importAliases: {},
    },
    () => {},
  )
  try {
    await fixture.file('app/src/main.js', 'export {};')
    await fixture.file(
      'app/config/devup.json',
      '{"tailwind":{"css":"styles/tailwind.css"}}',
    )
    await fixture.file('app/styles/tailwind.css', '@import "./brand.css";')
    await fixture.file(
      'app/styles/brand.css',
      '@theme { --color-brand: #112233; }',
    )
    activation.configure(
      await resolveConfig(
        {
          root: fixture.root,
          configFile: false,
          logLevel: 'silent',
          environments: {
            client: { consumer: 'client' },
            server: { consumer: 'server' },
          },
          build: {
            watch: {},
            lib: { entry: `${fixture.root}/src/main.js`, formats: ['es'] },
          },
        },
        'build',
      ),
    )
    await activation.prepare(() => {})
    await action(activation, fixture)
  } finally {
    activation.dispose()
    await fixture.close()
  }
}

function extract(id: string, code: string) {
  const output = wasm.codeExtract(
    id,
    code,
    '@devup-ui/react',
    'df',
    true,
    false,
    false,
    {},
    'compiled-mdx',
  )
  try {
    return { code: output.code, css: wasm.getCss(null, false) }
  } finally {
    output.free()
  }
}

it('applies the new Tailwind color rule in fresh JS from every refreshed context (UNIT)', async () => {
  await withOwner(async (activation, fixture) => {
    // Given the original two-context preparation and root-relative imported theme.
    for (const context of contexts) {
      activation.start(context)
      const modules = activation.modules(context)
      modules.direct('virtual:theme.mdx', 'virtual:theme.mdx', {
        code: themeCode,
        sourceType: 'compiled-mdx',
      })
      expect(modules.source('virtual:theme.mdx')).toBeDefined()
      const before = activation.run(context, () =>
        extract('virtual:theme.mdx', themeCode),
      )
      expect(before.css).toMatch(/background(?:-color)?:#(?:112233|123)\b/)
    }
    const numbers = wasm.exportFileMap()
    await fixture.file(
      'app/styles/brand.css',
      '@theme { --color-brand: #445566; }',
    )
    // When the real production owner refreshes its consumed theme.
    await activation.updateTheme()
    // Then fresh transformed JSX applies the independently specified new-color rule.
    for (const context of contexts) {
      expect(
        activation.modules(context).source('virtual:theme.mdx'),
      ).toBeUndefined()
      const fresh = activation.run(context, () =>
        extract('virtual:theme.mdx', themeCode),
      )
      const rule =
        /\.([\w-]+)\{background(?:-color)?:#(?:445566|456)\b[^}]*\}/.exec(
          fresh.css,
        )
      expect(rule?.[1]).toBeDefined()
      const classes = /className\s*=\s*["']([^"']+)["']/
        .exec(fresh.code)?.[1]
        ?.split(/\s+/)
      expect(classes).toContain(rule?.[1] ?? 'missing-new-color-class')
      expect(wasm.exportFileMap()).toBe(numbers)
    }
    expect(wasm.exportFileMap()).toBe(numbers)
  })
})

it('preserves live legacy sheet, maps and prefix when real theme refresh rejects mixed consumers (UNIT)', async () => {
  await withOwner(async (activation, fixture) => {
    // Given a prepared owner followed by a live, distinct legacy engine interval.
    activation.start('client')
    activation.run('client', () => extract('virtual:theme.mdx', themeCode))
    const legacy = { integration: 'Bun', root: `${fixture.root}/legacy` }
    const endLegacy = beginBuild(wasm, legacy)
    try {
      wasm.registerTheme({})
      wasm.registerShorthands({})
      wasm.setPrefix('legacy-')
      wasm.seedFileMap(['legacy.tsx'])
      const live = extract(
        'legacy.tsx',
        "import {Box} from '@devup-ui/react'; export const view=<Box w='713px'/>;",
      )
      expect(live.css).toContain('width:713px')
      const before = { ...productionEngine.capture(), prefix: wasm.getPrefix() }
      expect(before.files).toContain('legacy.tsx')
      expect(before.prefix).toBe('legacy-')
      await fixture.file(
        'app/styles/brand.css',
        '@theme { --color-brand: #445566; }',
      )
      // When real updateTheme reaches shared-engine admission in this same binding.
      await expect(activation.updateTheme()).rejects.toMatchObject({
        name: 'MixedBuildIntegrationError',
        requested: { integration: 'Vite', root: fixture.root },
        active: legacy,
        message: new MixedBuildIntegrationError(
          { integration: 'Vite', root: fixture.root },
          legacy,
        ).message,
      })
      // Then rejection is atomic for the live legacy engine, not just its error.
      expect({
        ...productionEngine.capture(),
        prefix: wasm.getPrefix(),
      }).toEqual(before)
      expect(wasm.getCss(null, false)).toBe(live.css)
    } finally {
      endLegacy()
    }
  })
})

it('keeps untouched sibling class and global CSS when another context refreshes the real theme (UNIT)', async () => {
  await withOwner(async (activation, fixture) => {
    // Given real server-context contributions that will not be re-extracted.
    activation.start('server')
    const sibling = activation.run('server', () =>
      extract(
        'virtual:sibling.mdx',
        "import {Box,globalCss} from '@devup-ui/react'; globalCss({'.refresh-sibling':{outlineOffset:'7px'}}); export const view=<Box w='719px'/>;",
      ),
    )
    const rule = /\.([\w-]+)\{width:719px\}/.exec(sibling.css)
    expect(rule?.[0]).toBeDefined()
    expect(sibling.css).toContain('.refresh-sibling{outline-offset:7px}')
    activation.start('client')
    activation.run('client', () => extract('virtual:theme.mdx', themeCode))
    const numbers = wasm.exportFileMap()
    await fixture.file(
      'app/styles/brand.css',
      '@theme { --color-brand: #445566; }',
    )
    // When the client refreshes without running the sibling extractor again.
    await activation.updateTheme()
    // Then the accumulated class rule and global rule survive in every context.
    for (const context of contexts) {
      const css = activation.run(context, () => wasm.getCss(null, false))
      expect(css).toContain(rule?.[0] ?? 'missing-sibling-rule')
      expect(css).toContain('.refresh-sibling{outline-offset:7px}')
      expect(wasm.exportFileMap()).toBe(numbers)
    }
  })
})
