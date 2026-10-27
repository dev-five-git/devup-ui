import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import type { Compiler, Configuration, Stats } from 'webpack'

import { DevupUIWebpackPlugin } from '../plugin'
import {
  cjsGuardCases,
  guardCases,
  mdxBuildFixture,
  mdxLanding,
} from './mdx-build-fixture'

const bundled: { webpack(config: Configuration): Compiler } = mdxLanding(
  'next/dist/compiled/webpack/webpack',
)

async function bundle(source: string, selected = false, extension = '.mdown') {
  wasm.resetBuildState()
  const fixture = mdxBuildFixture(source, extension)
  const compiler = bundled.webpack({
    context: fixture.root,
    mode: 'development',
    entry: fixture.filename,
    output: { path: `${fixture.root}/out` },
    resolve: { alias: fixture.alias },
    externals: {
      react: 'commonjs react',
      'react/jsx-runtime': 'commonjs react/jsx-runtime',
    },
    experiments: { css: true },
    optimization: { minimize: false },
    module: { rules: [fixture.rule, { test: /\.css$/, type: 'css' }] },
    plugins: [
      new DevupUIWebpackPlugin({
        singleCss: true,
        mdxExtensions: selected ? ['.mdx', '.mdown'] : undefined,
      }),
    ],
  })
  try {
    const stats = await new Promise<Stats>((done, reject) =>
      compiler.run((error, result) => {
        if (error) reject(error)
        else if (result) done(result)
        else reject(new Error('Missing build stats'))
      }),
    )
    return {
      errors: stats.toJson({ all: false, errors: true }).errors ?? [],
      css: wasm.getCss(null, false),
    }
  } finally {
    await new Promise<void>((done, reject) =>
      compiler.close((error) => (error ? reject(error) : done())),
    )
    fixture.cleanup()
  }
}

it('extracts custom MDX with the same isolated CSS as .mdx', async () => {
  const source = 'import {Box} from "@devup-ui/react"\n\n<Box bg="red" />'
  const standard = await bundle(source, true, '.mdx')
  const custom = await bundle(source, true)
  expect(custom.errors).toEqual([])
  expect(custom.css).toContain('red')
  expect(custom.css).toBe(standard.css)
})

it.each([...guardCases, ...cjsGuardCases])(
  'uses final dependencies to guard %s',
  async (_name, source, errors) => {
    const result = await bundle(source)
    expect(result.errors.length > 0).toBe(errors)
    if (errors) expect(result.errors[0]?.message).toContain('mdxExtensions')
  },
)

it('follows an actual forwarded Widget export to Box', async () => {
  const result = await bundle(
    'import {Widget} from "./barrel.js"\n\n<Widget />',
  )
  expect(result.errors[0]?.message).toContain('mdxExtensions')
})
