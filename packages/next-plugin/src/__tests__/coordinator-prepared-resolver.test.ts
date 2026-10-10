import { join } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import { createInput } from '../coordinator-ledger'
import type { PreparedSourceGeneration } from '../coordinator-options'
import { readCoordinatorState } from '../state'
import { createTestApp, removeTestApps } from './coordinator-app'
import {
  compiledSource,
  cssQuery,
  generation,
  preparedCore,
} from './coordinator-prepared-fixture'

afterEach(removeTestApps)

it.each(['css', 'extract'] as const)(
  'uses the refreshed generation resolver for imported compiled MDX values during %s',
  async (operation) => {
    // Given unchanged Markdown whose controlled compilation depends on an edited plugin input.
    const app = createTestApp()
    app.write('src/values.mdx', '# not JavaScript')
    const dependency = app.write('plugin-input.txt', 'old')
    const code =
      'import { Box } from "@devup-ui/react"; import { color } from "./values.mdx"; export const A = <Box bg={color} />'
    const ordinary = createInput(
      app.root,
      {
        filename: 'src/page.tsx',
        resourcePath: join(app.root, 'src/page.tsx'),
        code,
      },
      ['src/values.mdx'],
    )
    const old = compiledSource(app, {
      filename: 'src/values.mdx',
      code: 'export const color = "red"',
      dependencies: [dependency],
    })
    const staged: { generation?: PreparedSourceGeneration } = {}
    const prepared = await preparedCore(app, {
      generation: generation([old]),
      ordinaryInputs: [ordinary],
      prepareReplay: async ({ generation }) =>
        generation.sources[0]?.input.source === 'export const color = "green"'
          ? generation
          : (staged.generation ?? generation),
    })
    app.write('plugin-input.txt', 'new')
    staged.generation = generation([
      compiledSource(app, {
        filename: 'src/values.mdx',
        code: 'export const color = "green"',
        dependencies: [dependency],
      }),
    ])

    // When replay or an ordinary importing extraction runs before native MDX delivery.
    if (operation === 'css') await prepared.core.css(cssQuery)
    else
      await prepared.core.extract({
        filename: 'src/new.tsx',
        resourcePath: join(app.root, 'src/new.tsx'),
        code,
      })

    // Then both replay and importing requests read compiled values from the generation, never raw Markdown.
    expect((await prepared.core.css(cssQuery)).css).toContain(
      'background:green',
    )
    expect((await prepared.core.css(cssQuery)).css).not.toContain(
      'background:red',
    )
    expect(
      readCoordinatorState(prepared.options.stateFile ?? '', '')?.inputs.find(
        (input) => input.filename === 'src/values.mdx',
      )?.source,
    ).toBe('export const color = "green"')
    prepared.core.close()
  },
)
