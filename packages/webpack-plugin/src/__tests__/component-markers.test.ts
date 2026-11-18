import { relative } from 'node:path'

import {
  ComponentMarkerLedger,
  resetMarkerProcessState,
} from '@devup-ui/plugin-utils'
import { beforeEach, expect, it } from 'bun:test'
import type { Compilation, Compiler } from 'webpack'

import { stripComponentMarkers } from '../component-markers'

beforeEach(() => {
  resetMarkerProcessState()
})

class RawSource {
  constructor(private readonly code: string) {}
  source(): string {
    return this.code
  }
}

/** Webpack's ReplaceSource, as far as the plugin uses it */
class ReplaceSource {
  private readonly edits: [number, number][] = []
  constructor(private readonly original: RawSource) {}
  replace(start: number, end: number, text: string): void {
    expect(text).toBe('')
    this.edits.push([start, end])
  }
  source(): string {
    const code = this.original.source()
    let result = ''
    let at = 0
    for (const [start, end] of this.edits) {
      result += code.slice(at, start)
      at = end + 1
    }
    return result + code.slice(at)
  }
}

const webpack = {
  webpack: { WebpackError: Error, sources: { ReplaceSource } },
} as unknown as Pick<Compiler, 'webpack'>

type Asset = { name: string; source: RawSource }

function compilation(
  assets: Record<string, string>,
  modules: object[] = [],
): { compilation: Compilation; assets: Map<string, Asset>; warnings: Error[] } {
  const stored = new Map<string, Asset>(
    Object.entries(assets).map(([name, code]) => [
      name,
      { name, source: new RawSource(code) },
    ]),
  )
  const warnings: Error[] = []
  const value = {
    modules: new Set(modules),
    warnings,
    getAssets: () => [...stored.values()],
    updateAsset: (name: string, source: RawSource) => {
      stored.set(name, { name, source })
    },
  }
  return {
    compilation: value as unknown as Compilation,
    assets: stored,
    warnings,
  }
}

const root = process.cwd()
const module = (resource: string | undefined, code: string | undefined) => ({
  ...(resource === undefined ? {} : { resource }),
  originalSource: () => (code === undefined ? null : new RawSource(code)),
})

it('drops markers no module selects from script assets', () => {
  // Given
  const ledger = new ComponentMarkerLedger()
  ledger.scanned(['a--b'], ['src/A.tsx'])
  ledger.record({
    id: 'src/A.tsx',
    input: '',
    givenMarkers: ['a--a', 'a--b', 'a--c', 'a--d'],
    selectedMarkers: [],
  })
  const { compilation: build, assets } = compilation(
    {
      'main.js': 'f("a--a x"),f("a--b y"),f("a--c"),f("a--d"),f("a--e")',
      'other.mjs': 'f("y")',
      'main.css': '.a--a{}',
    },
    [
      module(`${root}/src/A.tsx`, 'unread'),
      module(`${root}/vendor.js`, 'f("a--c")'),
      module(undefined, 'f("a--d")'),
      module(`${root}/raw.bin`, undefined),
      {
        ...module(`${root}/src/Cached.tsx`, 'f("a--e")'),
        buildInfo: {
          devupComponentMarkers: {
            id: 'src/Cached.tsx',
            input: '',
            givenMarkers: ['a--e'],
            selectedMarkers: [],
          },
        },
      },
      { ...module(`${root}/src/A.tsx`, 'unread'), buildInfo: {} },
    ],
  )
  // When
  stripComponentMarkers(webpack, build, ledger, root, relative)
  // Then
  expect(assets.get('main.js')?.source.source()).toBe(
    'f("x"),f("a--b y"),f("a--c"),f("a--d"),f("")',
  )
  expect(assets.get('main.css')?.source.source()).toBe('.a--a{}')
  expect(assets.get('other.mjs')?.source.source()).toBe('f("y")')
})

it('leaves the assets alone when nothing is left to drop', () => {
  // Given
  const ledger = new ComponentMarkerLedger()
  ledger.scanned([], [])
  const {
    compilation: build,
    assets,
    warnings,
  } = compilation({
    'main.js': 'f("a--a")',
  })
  // When
  stripComponentMarkers(webpack, build, ledger, root, relative)
  // Then
  expect(assets.get('main.js')?.source.source()).toBe('f("a--a")')
  expect(warnings).toEqual([])
})

it('keeps every marker and warns when the scan missed a selecting module', () => {
  // Given
  const ledger = new ComponentMarkerLedger()
  ledger.scanned([], [])
  ledger.record({
    id: 'src/A.tsx',
    input: '',
    givenMarkers: ['a--a'],
    selectedMarkers: [],
  })
  ledger.record({
    id: 'late.tsx',
    input: '',
    givenMarkers: [],
    selectedMarkers: ['a--a'],
    cause: 'context-module',
  })
  const {
    compilation: build,
    assets,
    warnings,
  } = compilation({
    'main.js': 'f("a--a")',
  })
  // When
  stripComponentMarkers(webpack, build, ledger, root, relative)
  // Then
  expect(assets.get('main.js')?.source.source()).toBe('f("a--a")')
  expect(warnings.map(({ message }) => message)).toEqual([
    expect.stringContaining('late.tsx [context-module]'),
  ])
})

it('keeps every marker quietly without a scan', () => {
  // Given
  const ledger = new ComponentMarkerLedger()
  const { compilation: build, warnings } = compilation({ 'main.js': '' })
  // When
  stripComponentMarkers(webpack, build, ledger, root, relative)
  // Then
  expect(warnings).toEqual([])
})
