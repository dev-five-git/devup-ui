import { expect, it } from 'bun:test'

import { productionFixture } from './production-fixture'

it.each([false, true])(
  'refuses synchronous Markdown values in a native build when singleCss=%s',
  async (singleCss) => {
    // Given: a real authored Markdown value used by a compile-time style call.
    const f = await productionFixture()
    try {
      await f.file('app/devup.json', '{}')
      await f.file(
        'app/src/content.mdx',
        "export const color='red';\n\n# Content",
      )
      await f.file(
        'app/src/main.js',
        "import {css} from '@devup-ui/react'; import {color} from './content.mdx'; export const cls=css({color});",
      )
      // When: Vite runs the source plugin with no synchronous MDX compiler cache.
      const result = f.run(singleCss, [
        {
          name: 'W20bq-project-markdown-compiler',
          transform(_code, id) {
            if (id.endsWith('/content.mdx'))
              return "export const color='red'; export default function Content(){}"
          },
        },
      ])
      // Then: the actual resolver boundary refuses, rather than reading raw prose.
      await expect(result).rejects.toThrow(
        'Markdown values have no synchronous compiler input; move the imported value into JS/TS',
      )
    } finally {
      await f.close()
    }
  },
)

it.each([false, true])(
  'fails native final verification on unowned metadata when singleCss=%s',
  async (singleCss) => {
    // Given: a genuine native module with no extraction obligations.
    const f = await productionFixture()
    let transported = false
    try {
      await f.file('app/devup.json', '{}')
      const entry = await f.file('app/src/main.js', 'export const value=1;')
      // When: a public module hook supplies invalid unowned metadata.
      const result = f.run(singleCss, [
        {
          name: 'W20bq-public-metadata-boundary',
          moduleParsed(info) {
            if (info.id === entry) {
              info.meta['devupDeferredDefaults'] = '{}'
              transported = true
            }
          },
        },
      ])
      // Then: buildEnd fails located through production cleanup, not native parsing.
      await expect(result).rejects.toThrow(
        /main\.js:1:1: deferred default metadata cannot use/,
      )
      expect(transported).toBe(true)
    } finally {
      await f.close()
    }
  },
)
