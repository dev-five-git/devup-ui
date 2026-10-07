import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxSourceManager } from '../mdx-source-generation'
import { createWasm, extractWithModuleResolver } from '../wasm'
import { sourceFixture } from './mdx-source-fixture'

it.each([
  ['md', true, true, 'react/jsx-dev-runtime', '_jsxDEV'],
  ['md', true, false, 'react/jsx-dev-runtime', '_jsxDEV'],
  ['mdx', true, true, 'react/jsx-dev-runtime', '_jsxDEV'],
  ['mdx', true, false, 'react/jsx-dev-runtime', '_jsxDEV'],
  ['mdx', false, true, 'react/jsx-runtime', '_jsx'],
] as const)(
  'extracts installed automatic-runtime .%s when development=%s and sourceMap=%s',
  async (extension, development, sourceMap, runtime, call) => {
    // Given: caller-authored options precede preparation, not a compiler override.
    const filename = `app/page.${extension}`
    const f = sourceFixture(
      {
        [filename]: `import { Box } from '@devup-ui/react'

# Development styles

<Box bg="red" p={4} _hover={{ bg: 'blue' }} />
`,
      },
      {},
      development,
    )
    f.compilerOptions.jsx = false
    const manager = createMdxSourceManager({
      ...f.binding,
      async selectPipeline(path, signal) {
        const selected = await f.binding.selectPipeline(path, signal)
        return selected
          ? { ...selected, context: { ...selected.context, sourceMap } }
          : undefined
      },
    })

    // When: preparation executes the installed chain before real WASM extraction.
    const generation = await manager.prepare(f.signal)
    const engine = createWasm(resolve(import.meta.dir, '../../../..'))
    generation.configureWasm(engine)

    // Then: one resource compile, original IDs, actual runtime calls and static CSS.
    expect(f.counts()).toBe(1)
    expect(generation.pendingOrdinary).toEqual([])
    expect(Object.keys(generation.compiled)).toEqual([join(f.root, filename)])
    expect(generation.sources.map(({ input }) => input.filename)).toEqual([
      filename,
    ])
    for (const { input } of generation.sources) {
      expect(input.source).toContain(runtime)
      expect(input.source).toContain(`${call}(Box,`)
      const prepared = generation.compiled[join(f.root, filename)]?.prepared
      expect(prepared?.filename).toBe(join(f.root, filename))
      if (sourceMap) expect(prepared?.map).toBeDefined()
      else expect(prepared?.map).toBeUndefined()

      using output = extractWithModuleResolver(engine, sourceMap, [
        input.filename,
        input.source,
        '@devup-ui/react',
        './df',
        true,
        false,
        false,
        {},
      ])
      expect(output.code).toContain(`${call}("div",`)
      expect(output.code).toContain('className:')
      expect(output.code).not.toContain('@devup-ui/react')
      expect(output.code).not.toMatch(/\bBox\b/)
      expect(output.code).not.toMatch(/\b(?:bg|p|_hover|style):/)
      if (sourceMap) expect(output.map).toBeDefined()
      else expect(output.map).toBeUndefined()
    }
    expect(engine.getCss(null, false)).toContain('background:red')
    expect(engine.getCss(null, false)).toContain('padding:16px')
    expect(engine.getCss(null, false)).toMatch(/:hover\{background:blue\}/)
    expect(f.counts()).toBe(1)
  },
)
