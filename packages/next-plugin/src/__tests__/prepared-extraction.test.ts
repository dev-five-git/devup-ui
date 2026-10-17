import { join } from 'node:path'

import { afterEach, expect, it, mock, spyOn } from 'bun:test'

import {
  buildEngine,
  extractInput,
  extractRequest,
} from '../coordinator-engine'
import { createEngineConfigurer } from '../engine-config'
import { createAppContext } from '../session'
import { exportAllocatorState } from '../state'
import { createWasm, withModuleResolver } from '../wasm'
import { box, installProjectHooks, makeProject } from './project'

installProjectHooks()
afterEach(() => {
  mock.restore()
})

const plan = { canonicalMap: {}, fileRoutes: {}, atomThreshold: null }
const invalid = `import { css } from '@devup-ui/react'\nconst v = Math.random()\nexport const c = css({ bg: v })`

function fixture(sourceMap: boolean) {
  const root = makeProject({
    'src/page.mdx': '# raw',
    'src/value.mdx': '# raw',
  })
  process.chdir(root)
  const context = { ...createAppContext({}, {}), sourceMap }
  const cache = new Map([
    [
      join(root, 'src/page.mdx'),
      {
        code: invalid,
        map: {
          version: 3,
          sources: [join(root, 'src/page.mdx')],
          mappings: ';;AAKA',
          names: [],
        },
      },
    ],
    [
      join(root, 'src/value.mdx'),
      {
        code: 'export const color = "purple"',
        map: {
          version: 3,
          sources: [join(root, 'src/value.mdx')],
          mappings: 'AAOA',
          names: [],
        },
      },
    ],
  ])
  const resolver = {
    prepareSource: (file: string) => cache.get(file),
    includeMdx: true,
    alias: { value: join(root, 'src/value.mdx') },
  }
  const configure = createEngineConfigurer(context, {
    theme: {},
    plan,
    resolver,
  })
  const engine = createWasm(root)
  configure(engine)
  const settings = {
    package: context.libPackage,
    cssDir: context.cssDir,
    singleCss: true,
    sourceMap,
    importAliases: {},
  }
  const input = {
    filename: 'src/page.mdx',
    resourcePath: join(root, 'src/page.mdx'),
    source: invalid,
  }
  return { root, cache, resolver, configure, engine, settings, input }
}

it.each([true, false])(
  'remaps real WASM own-source errors before RPC location handling with maps=%s',
  (sourceMap) => {
    // Given prepared JavaScript under its actual Markdown filename.
    const f = fixture(sourceMap)
    // When the common request seam extracts an invalid static css value.
    const action = () =>
      extractRequest(f.engine, f.settings, { ...f.input, code: f.input.source })
    // Then the compiler map locates the failure in Markdown.
    expect(action).toThrow(`${join(f.root, 'src/page.mdx')}:6:1:`)
  },
)

it.each([true, false])(
  'remaps imported extractor-format locations with maps=%s',
  (sourceMap) => {
    // Given a real resolver/WASM import has registered the imported map.
    const f = fixture(sourceMap)
    const source = `import { Box } from '@devup-ui/react'; import { color } from 'value'; export const C = <Box bg={color} />`
    extractInput(f.engine, f.settings, {
      ...f.input,
      filename: 'src/main.tsx',
      source,
    })
    const cause = new Error('src/value.mdx:1:1: invalid imported value')
    const method = sourceMap ? 'codeExtract' : 'codeExtractWithoutSourceMap'
    spyOn(f.engine, method).mockImplementation(() => {
      throw cause
    })
    // When #757-style imported positions are emitted (synthetic).
    const action = () =>
      extractInput(f.engine, f.settings, {
        ...f.input,
        source: box('bg="red"'),
      })
    // Then the shared resolver translates the imported location.
    expect(action).toThrow(`${join(f.root, 'src/value.mdx')}:8:1:`)
  },
)

it.each([true, false])(
  'retains original errors for an ordinary engine with maps=%s',
  (sourceMap) => {
    // Given an ordinary configured engine and a non-Error WASM throw.
    const f = fixture(sourceMap)
    const cause = Object.freeze({ message: 'raw failure' })
    withModuleResolver(f.engine, f.root)
    spyOn(
      f.engine,
      sourceMap ? 'codeExtract' : 'codeExtractWithoutSourceMap',
    ).mockImplementation(() => {
      throw cause
    })
    // When extraction fails without a prepared hook.
    try {
      extractInput(f.engine, f.settings, f.input)
    } catch (error) {
      // Then the seam does not wrap or remap ordinary exceptions.
      expect(error).toBe(cause)
      return
    }
    throw new Error('Expected extraction failure')
  },
)

it('keeps resolver maps across repeated configuration of the same engine', () => {
  // Given imported maps registered through real extraction.
  const f = fixture(false)
  extractInput(f.engine, f.settings, {
    ...f.input,
    source: `import { Box } from '@devup-ui/react'; import { color } from 'value'; export const C = <Box bg={color} />`,
  })
  f.configure(f.engine)
  spyOn(f.engine, 'codeExtractWithoutSourceMap').mockImplementation(() => {
    throw new Error('src/value.mdx:1:1: failure')
  })
  // When another extraction uses the retained configured engine.
  const action = () => extractInput(f.engine, f.settings, f.input)
  // Then configuration has not lost the imported map.
  expect(action).toThrow(`${join(f.root, 'src/value.mdx')}:8:1:`)
})

it.each([true, false])(
  'configures replacement engines with prepared values and remaps rebuild errors with maps=%s',
  (sourceMap) => {
    // Given a generation configurer and allocator from the live engine.
    const f = fixture(sourceMap)
    const input = { ...f.input, dependencies: [], stamps: {}, backing: '' }
    // When transactional replay builds a replacement engine.
    const action = () =>
      buildEngine({
        createEngine: () => createWasm(f.root),
        live: f.engine,
        configure: f.configure,
        allocator: exportAllocatorState(f.engine),
        theme: undefined,
        settings: f.settings,
        inputs: [input],
      })
    // Then rebuild reporting has already received the remapped error.
    expect(action).toThrow(`${join(f.root, 'src/page.mdx')}:6:1:`)
  },
)

it('uses new generation maps and labels prepared output without a map', () => {
  // Given a new generation whose prepared source has no map.
  const f = fixture(false)
  withModuleResolver(f.engine, f.root, { prepareSource: () => invalid })
  // When direct extraction fails.
  const action = () => extractInput(f.engine, f.settings, f.input)
  // Then stale maps from the previous resolver are not used.
  expect(action).toThrow('(in compiled output)')
})

it.each([undefined, Promise.resolve('')])(
  'blocks missing or asynchronous preparation before extraction',
  (prepared) => {
    // Given a cache reader that cannot synchronously provide Markdown.
    const f = fixture(false)
    withModuleResolver(f.engine, f.root, { prepareSource: () => prepared })
    // When extraction attempts to consume that required module.
    const action = () => extractInput(f.engine, f.settings, f.input)
    // Then no raw Markdown reaches WASM.
    expect(action).toThrow('prepare Markdown before extraction')
  },
)
