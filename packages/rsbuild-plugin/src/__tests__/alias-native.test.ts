import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { createRsbuild } from '@rsbuild/core'
import { expect, it } from 'bun:test'

import { aliasFixture } from '../../../plugin-utils/src/__tests__/alias-array-fixture'
import { mdxLanding } from '../../../webpack-plugin/src/__tests__/mdx-build-fixture'
import { DevupUI } from '../plugin'

it.each(['false', 'later-false', 'first-success', 'empty'])(
  'passes the actual Rspack alias map when %s',
  async (kind) => {
    const fixture = aliasFixture()
    const { root, file } = fixture
    const entry = file(
      'src/main.js',
      'import {css} from "@devup-ui/react"; import {value} from "provider"; export const cls=css({color: value === undefined ? "red" : value})',
    )
    const blue = file('outside/blue.js', 'export const value="blue"')
    file('node_modules/provider/package.json', '{"main":"index.js"}')
    file('node_modules/provider/index.js', 'export const value="blue"')
    const target: string | false | (string | false)[] =
      kind === 'false'
        ? false
        : kind === 'later-false'
          ? [join(root, 'missing'), false]
          : kind === 'empty'
            ? []
            : [blue, false]
    wasm.resetBuildState()
    try {
      const builder = await createRsbuild({
        cwd: root,
        rsbuildConfig: {
          mode: 'development',
          plugins: [DevupUI({ singleCss: true, include: ['provider'] })],
          source: { entry: { main: entry } },
          resolve: {
            alias: {
              provider: target,
              '@devup-ui/react': mdxLanding.resolve('@devup-ui/react'),
            },
          },
          output: {
            target: 'node',
            distPath: { root: join(root, 'out') },
            minify: false,
          },
        },
      })
      const result = await builder.build()
      try {
        expect(wasm.getCss(null, false)).toContain(
          kind === 'false' || kind === 'later-false'
            ? 'color:red'
            : 'color:blue',
        )
      } finally {
        await result.close()
      }
    } finally {
      fixture.dispose()
    }
  },
)
