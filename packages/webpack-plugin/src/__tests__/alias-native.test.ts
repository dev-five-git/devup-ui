import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import type { Compiler, Configuration, Stats } from 'webpack'

import { aliasFixture } from '../../../plugin-utils/src/__tests__/alias-array-fixture'
import { DevupUIWebpackPlugin } from '../plugin'
import { mdxLanding } from './mdx-build-fixture'

const bundled: { webpack(config: Configuration): Compiler } = mdxLanding(
  'next/dist/compiled/webpack/webpack',
)

it.each([false, true])(
  'passes native descriptors through graph, setup and loader when watch=%s',
  async (watch) => {
    const fixture = aliasFixture()
    const { root, file } = fixture
    const entry = file(
      'src/main.js',
      'import {css} from "@devup-ui/react"; import {value} from "provider"; export const cls=css({color: value === undefined ? "red" : value})',
    )
    const blue = file('outside/blue.js', 'export const value="blue"')
    try {
      for (const target of [false, blue] as const) {
        wasm.resetBuildState()
        const compiler = bundled.webpack({
          context: root,
          mode: 'development',
          entry,
          target: 'node',
          output: { path: join(root, 'out'), library: { type: 'commonjs2' } },
          resolve: {
            alias: [
              { name: 'provider', alias: target },
              { name: 'provider', alias: false },
              {
                name: '@devup-ui/react',
                alias: mdxLanding.resolve('@devup-ui/react'),
              },
            ],
          },
          experiments: { css: true },
          module: { rules: [{ test: /\.css$/, type: 'css' }] },
          plugins: [new DevupUIWebpackPlugin({ singleCss: true, watch })],
        })
        try {
          const stats = await new Promise<Stats>((done, reject) =>
            compiler.run((error, result) => {
              if (error) reject(error)
              else if (result) done(result)
              else reject(new Error('Missing build stats'))
            }),
          )
          expect(
            stats.toJson({ all: false, errors: true }).errors ?? [],
          ).toEqual([])
          const result: { exports: { cls?: string } } = { exports: {} }
          new Function(
            'module',
            'exports',
            readFileSync(join(root, 'out/main.js'), 'utf8'),
          )(result, result.exports)
          expect(wasm.getCss(null, false)).toContain(
            `.${result.exports.cls}{color:${target === false ? 'red' : 'blue'}}`,
          )
        } finally {
          await new Promise<void>((done, reject) =>
            compiler.close((error) => (error ? reject(error) : done())),
          )
        }
      }
    } finally {
      fixture.dispose()
    }
  },
)
