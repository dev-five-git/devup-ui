import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { createRsbuild } from '@rsbuild/core'
import { expect, it } from 'bun:test'

import {
  cjsGuardCases,
  guardCases,
  mdxBuildFixture,
} from '../../../webpack-plugin/src/__tests__/mdx-build-fixture'
import { DevupUI } from '../plugin'

async function bundle(source: string, selected = false, extension = '.mdown') {
  wasm.resetBuildState()
  const fixture = mdxBuildFixture(source, extension)
  const cjsIds: unknown[] = []
  try {
    const builder = await createRsbuild({
      cwd: fixture.root,
      rsbuildConfig: {
        mode: 'development',
        plugins: [
          DevupUI({
            singleCss: true,
            mdxExtensions: selected ? ['.mdx', '.mdown'] : undefined,
          }),
        ],
        source: { entry: { main: fixture.filename } },
        resolve: { alias: fixture.alias },
        output: {
          target: 'node',
          distPath: { root: join(fixture.root, 'out') },
          minify: false,
        },
        tools: {
          rspack: {
            module: { rules: [fixture.rule] },
            plugins: [
              {
                apply(compiler) {
                  compiler.hooks.thisCompilation.tap(
                    'PublicCjsIds',
                    (compilation) => {
                      compilation.hooks.finishModules.tap(
                        'PublicCjsIds',
                        (modules) => {
                          for (const module of modules) {
                            if (module.resource !== fixture.filename) continue
                            for (const dependency of module.dependencies)
                              if (dependency.type.startsWith('cjs'))
                                cjsIds.push(dependency.ids)
                          }
                        },
                      )
                    },
                  )
                },
              },
            ],
            externals: {
              react: 'commonjs react',
              'react/jsx-runtime': 'commonjs react/jsx-runtime',
            },
          },
        },
      },
    })
    const result = await builder.build()
    try {
      return { css: wasm.getCss(null, false), cjsIds }
    } finally {
      await result.close()
    }
  } finally {
    fixture.cleanup()
  }
}

it('extracts custom MDX with the same isolated CSS as .mdx', async () => {
  const source = 'import {Box} from "@devup-ui/react"\n\n<Box bg="red" />'
  const expected = await bundle(source, true, '.mdx')
  const custom = await bundle(source, true)
  expect(custom.css).toContain('red')
  expect(custom.css).toBe(expected.css)
})

it.each(guardCases)(
  'uses Rspack definitely-used IDs to guard %s',
  async (name, source, errors) => {
    const action = bundle(source)
    if (errors && name !== 'namespace destructure')
      await expect(action).rejects.toThrow()
    else expect((await action).css).not.toContain('{')
  },
)

it.each(cjsGuardCases)(
  'allows require uses with opaque public Rspack IDs when %s',
  async (name, source) => {
    const result = await bundle(source)

    expect(result.cjsIds).toEqual(
      name === 'require shadowed' || name === 'dynamic opaque'
        ? []
        : [undefined],
    )
    expect(result.css).not.toContain('{')
  },
)

it('allows opaque destructuring while definite member IDs remain guarded', async () => {
  // Rspack gives these destructuring uses the same empty export-ID list.
  for (const name of ['css', 'getTheme']) {
    const source = `import * as DU from "@devup-ui/react"\n\nexport const {${name}: value} = DU\n\n# Plain`
    expect((await bundle(source)).css).not.toContain('{')
  }
  await expect(
    bundle(
      'import * as DU from "@devup-ui/react"\n\nexport const value = DU.css({color:"red"})\n\n# Plain',
    ),
  ).rejects.toThrow('Rspack build failed')
})
