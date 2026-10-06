import { join } from 'node:path'

import { expect, it } from 'bun:test'

import {
  createMdxSourceManager,
  MdxSourceExtensionError,
} from '../mdx-source-generation'
import { createWasm, extractWithModuleResolver } from '../wasm'
import { sourceFixture, styledMdx } from './source-type-fixture'

it('locates an uncertified extension predicate and explains the finite suffix fix', () => {
  const predicate = '/\\.m(down)?$/'
  const error = new MdxSourceExtensionError('/next.config.mjs', predicate)

  expect(error.name).toBe('MdxSourceExtensionError')
  expect(error.configFile).toBe('/next.config.mjs')
  expect(error.extension).toBe(predicate)
  expect(error.message).toBe(
    `/next.config.mjs:1:1: MDX extension predicate ${predicate} needs a certified finite anchored literal suffix condition; use a literal extension or finite extension alternation`,
  )
})

it('extracts real configured JSX-preserving mdown roots under original identities', async () => {
  // Given
  const f = sourceFixture(
    { 'app/page.mdown': styledMdx },
    { extensions: ['.mdown'] },
  )
  const manager = createMdxSourceManager({
    ...f.binding,
    effectiveAppContext: {
      ...f.binding.effectiveAppContext,
      pageExtensions: ['mdown', 'tsx'],
    },
  })
  // When
  const generation = await manager.prepare(f.signal)
  // Then
  expect(generation.sources[0]?.input.filename).toBe('app/page.mdown')
  expect(generation.sources[0]?.input.sourceType).toBe('compiled-mdx')
  expect(generation.cacheReader(join(f.root, 'app/page.mdown'))).toMatchObject({
    sourceType: 'compiled-mdx',
  })
  expect(f.compilerOptions.jsx).toBe(true)
  expect(f.css(generation)).toContain('background:red')
})

it('evaluates a configured mdown export through the original aliased importer', async () => {
  // Given
  const f = sourceFixture(
    {
      'app/page.tsx': `import { css } from '@devup-ui/react'; import { color } from 'value'; export const style = css({color})`,
      'app/value.mdown': `export const color = 'blue'\n\n# Value`,
    },
    { extensions: ['.mdown'] },
  )
  const manager = createMdxSourceManager({
    ...f.binding,
    aliases: { ...f.binding.aliases, value$: join(f.root, 'app/value.mdown') },
  })
  // When
  const generation = await manager.prepare(f.signal)
  // Then
  expect(generation.sources[0]?.input.filename).toBe('app/value.mdown')
  expect(
    generation.ordinaryInputs.every((input) => input.sourceType === undefined),
  ).toBe(true)
  expect(f.css(generation)).toContain('color:blue')
})

it('retains mode and original identity when real compiler output is restored from restart cache', async () => {
  // Given
  const f = sourceFixture(
    {
      'app/page.tsx': `import './value.mdown'; export default ()=>null`,
      'app/value.mdown': styledMdx,
    },
    { extensions: ['.mdown'] },
  )
  const first = await f.manager.prepare(f.signal)
  const cached = createMdxSourceManager(
    f.binding,
    f.manager.restartCache(first),
  )
  // When
  const restored = await cached.prepare(f.signal)
  // Then
  expect(f.counts()).toBe(1)
  expect(restored.cacheReader(join(f.root, 'app/value.mdown'))).toMatchObject({
    sourceType: 'compiled-mdx',
  })
  expect(f.css(restored)).toContain('background:red')
})

it('locates unselected custom Markdown before it reaches WASM', async () => {
  // Given
  const f = sourceFixture(
    {
      'app/page.tsx': `import './value.mdown'; export default ()=>null`,
      'app/value.mdown': '# Raw',
    },
    { extensions: ['.mdown'], selectPipeline: async () => undefined },
  )
  // When / Then
  await expect(f.manager.prepare(f.signal)).rejects.toThrow(
    `${join(f.root, 'app/page.tsx')}:1:1: module source preparation`,
  )
})

it.each(['# Raw Markdown', 'export const color: string = "blue"'])(
  'rejects raw or TypeScript input claimed as compiled JavaScript JSX: %s',
  (code) => {
    // Given
    const f = sourceFixture({})
    const engine = createWasm(f.root)
    // When / Then
    expect(() =>
      extractWithModuleResolver(engine, false, [
        'value.mdown',
        code,
        '@devup-ui/react',
        './df',
        true,
        false,
        false,
        {},
        'compiled-mdx',
      ]),
    ).toThrow()
  },
)
