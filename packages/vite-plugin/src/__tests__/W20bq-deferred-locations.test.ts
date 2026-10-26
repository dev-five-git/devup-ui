import assert from 'node:assert/strict'

import { it } from 'bun:test'

import { productionFixture } from './production-fixture'

for (const singleCss of [false, true]) {
  it.each([true, false])(
    `locates deferred MDX refusal when mapped=%s and singleCss=${singleCss}`,
    async (mapped) => {
      // Given: an authored line-5 import moved to generated line 20 by a compiler.
      const f = await productionFixture()
      try {
        await f.file('app/devup.json', '{}')
        const authored = "\n\n\n\nimport Doc from './leaf.mdx';\n\n<Doc />\n"
        const consumer = await f.file('app/src/consumer.mdx', authored)
        await f.file('app/src/leaf.mdx', '# Not the native target')
        const leaf = await f.file(
          'app/src/leaf.js',
          'export default function Leaf(){return null;}',
        )
        const runtime = await f.file(
          'app/src/runtime.js',
          'export function jsx(type,props){return {type,props};}',
        )
        await f.file(
          'app/src/main.js',
          "import Consumer from './consumer.mdx'; import {jsx} from 'react/jsx-runtime'; export const view=jsx(Consumer,{});",
        )
        const generated =
          '// compiler padding\n'.repeat(19) +
          "import Doc from './leaf.mdx';\n" +
          "import {jsx} from 'react/jsx-runtime';\n" +
          'export default function Consumer(){return jsx(Doc,{});}\n'

        // When: the native resolver selects callable JS, without an owned MDX pass.
        const result = f.run(
          singleCss,
          [
            {
              name: 'W20bq-location-project-compiler',
              transform(_code, id) {
                if (id !== consumer) return
                return {
                  code: generated,
                  map: {
                    version: 3,
                    sources: [consumer],
                    sourcesContent: [authored],
                    names: [],
                    // OAIO: generated col 7 -> source 0, authored line 4, col 7.
                    mappings: mapped ? `${';'.repeat(19)}OAIO` : '',
                  },
                }
              },
            },
          ],
          {
            resolve: {
              alias: { './leaf.mdx': leaf, 'react/jsx-runtime': runtime },
            },
          },
        )

        // Then: refusal stays fail-closed, but its location is mapped or explicit.
        await assert.rejects(result, (error: unknown) => {
          assert(error instanceof Error)
          assert.match(
            error.message,
            /deferred default cannot use.*forwarding-unproven/,
          )
          const location = mapped
            ? `${consumer}:5:8:`
            : `${consumer}:20:8 (in compiled MDX):`
          assert.ok(error.message.includes(location), error.message)
          return true
        })
      } finally {
        await f.close()
      }
    },
  )

  it(`keeps authored JS coordinates when singleCss=${singleCss}`, async () => {
    // Given: a raw JS render-only read at line 5, column 8.
    const f = await productionFixture()
    try {
      await f.file('app/devup.json', '{}')
      await f.file('app/src/leaf.mdx', '# Not the native target')
      const leaf = await f.file(
        'app/src/leaf.js',
        'export default function Leaf(){return null;}',
      )
      const runtime = await f.file(
        'app/src/runtime.js',
        'export function jsx(type,props){return {type,props};}',
      )
      const entry = await f.file(
        'app/src/main.js',
        "\n\n\n\nimport Doc from './leaf.mdx'; import {jsx} from 'react/jsx-runtime'; import {css} from '@devup-ui/react'; export const cls=css({color:'red'}); export const view=jsx(Doc,{});",
      )

      // When: a public resolver redirects the MDX request to ordinary callable JS.
      const result = f.run(singleCss, [], {
        resolve: {
          alias: { './leaf.mdx': leaf, 'react/jsx-runtime': runtime },
        },
      })

      // Then: the same provenance refusal retains unqualified authored coordinates.
      await assert.rejects(result, (error: unknown) => {
        assert(error instanceof Error)
        assert.match(
          error.message,
          /deferred default cannot use.*forwarding-unproven/,
        )
        assert.ok(error.message.includes(`${entry}:5:8:`), error.message)
        assert.doesNotMatch(error.message, /in compiled MDX/)
        return true
      })
    } finally {
      await f.close()
    }
  })
}
