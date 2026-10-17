import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createEngineConfigurer } from '../engine-config'
import { type PreparedPrewarmInput, runPrewarm } from '../prewarm-run'
import { createAppContext } from '../session'
import { createWasm } from '../wasm'
import { box, installProjectHooks, makeProject } from './project'

installProjectHooks()
const plan = { canonicalMap: {}, fileRoutes: {}, atomThreshold: null }

it.each([true, false])(
  'prewarms exact prepared bytes and preserves compiler evidence with maps=%s',
  (sourceMap) => {
    // Given raw Markdown and complete compiled inputs with compiler evidence.
    const root = makeProject({
      'src/page.mdx': '# raw',
      'src/value.mdx': '# raw',
    })
    process.chdir(root)
    const context = { ...createAppContext({}, { singleCss: true }), sourceMap }
    const engine = createWasm(root)
    const map = {
      version: 3,
      sources: ['page.mdx'],
      mappings: 'AAAA',
      names: [],
    }
    const code = `import { Box } from '@devup-ui/react'; import { color } from './value.mdx'; export const C = <Box bg={color} />`
    const preparedInputs = new Map<string, PreparedPrewarmInput>([
      ['src/page.mdx', { code, map, dependencies: ['compiler-plugin.js'] }],
      ['src/value.mdx', { code: 'export const color = "green"' }],
    ])
    const cache = new Map(
      [...preparedInputs].map(([file, input]) => [join(root, file), input]),
    )
    createEngineConfigurer(context, {
      theme: {},
      plan,
      resolver: { prepareSource: (file) => cache.get(file) },
    })(engine)
    // When prewarm extracts under the original resource name.
    const result = runPrewarm({
      context,
      engine,
      files: ['src/page.mdx'],
      collectMs: undefined,
      preparedInputs,
    })
    // Then compiled source, compiler evidence, dependencies and CSS survive.
    expect(result.outputs.get('src/page.mdx')).toMatchObject({
      source: code,
      dependencies: ['src/value.mdx', 'compiler-plugin.js'],
    })
    expect(result.preparedInputs?.get('src/page.mdx')?.map).toBe(map)
    expect(engine.getCss(undefined, false)).toContain('background:green')
  },
)

it.each(['development', 'production'] as const)(
  'blocks incomplete preparation rather than reading raw Markdown in %s',
  (phase) => {
    // Given required preparation with no cached bytes for the selected file.
    const root = makeProject({ 'src/page.mdx': '# raw' })
    process.chdir(root)
    const context = { ...createAppContext({}, {}), phase }
    const engine = createWasm(root)
    // When prewarm consumes the incomplete generation.
    const action = () =>
      runPrewarm({
        context,
        engine,
        files: ['src/page.mdx'],
        collectMs: undefined,
        preparedInputs: new Map(),
      })
    // Then development and production both reject the located failure.
    expect(action).toThrow(`${join(root, 'src/page.mdx')}:1:1:`)
  },
)

it.each([true, false])(
  'blocks remapped preparation extraction failures even in development with maps=%s',
  (sourceMap) => {
    // Given required compiled JavaScript with a real WASM build-time failure.
    const root = makeProject({ 'src/page.mdx': '# raw' })
    process.chdir(root)
    const context = {
      ...createAppContext({}, {}),
      sourceMap,
      phase: 'development' as const,
    }
    const engine = createWasm(root)
    const code = `import { css } from '@devup-ui/react'\nconst v = Math.random()\nexport const c = css({ bg: v })`
    const prepared = {
      code,
      map: {
        version: 3,
        sources: [join(root, 'src/page.mdx')],
        mappings: ';;AAKA',
        names: [],
      },
    }
    createEngineConfigurer(context, {
      theme: {},
      plan,
      resolver: { prepareSource: () => prepared },
    })(engine)
    // When prewarm encounters that error.
    const action = () =>
      runPrewarm({
        context,
        engine,
        files: ['src/page.mdx'],
        collectMs: undefined,
        preparedInputs: new Map([['src/page.mdx', prepared]]),
      })
    // Then the blocking error is remapped before prewarm reports it.
    expect(action).toThrow(`${join(root, 'src/page.mdx')}:6:1:`)
  },
)

it('accepts empty prepared code without falling back to raw disk source', () => {
  // Given valid empty prepared output over a styled raw file.
  const root = makeProject({ 'src/page.tsx': box('bg="red"') })
  process.chdir(root)
  const context = createAppContext({}, {})
  const engine = createWasm(root)
  // When complete prepared prewarm runs.
  const result = runPrewarm({
    context,
    engine,
    files: ['src/page.tsx'],
    collectMs: undefined,
    preparedInputs: new Map([['src/page.tsx', { code: '' }]]),
  })
  // Then raw styles are not extracted.
  expect(result.outputs.get('src/page.tsx')?.source).toBe('')
  expect(engine.getCss(undefined, false)).not.toContain('background:red')
})
