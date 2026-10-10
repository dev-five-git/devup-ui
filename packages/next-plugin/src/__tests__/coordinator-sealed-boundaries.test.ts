import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import type { CoordinatorOptions } from '../coordinator-options'
import { readCoordinatorState } from '../state'
import { createTestApp, removeTestApps } from './coordinator-app'

const box = (color: string) =>
  `import { Box } from '@devup-ui/react'; export const C = () => <Box color="${color}" />;`
const query = { wait: true, importMainCss: false }

function fixture(extra: Partial<CoordinatorOptions> = {}) {
  const app = createTestApp()
  const stateFile = join(app.root, 'df/state.json')
  const wasm = extra.wasm ?? app.engine()
  extra.configureWasm?.(wasm)
  const core = createCore(
    app.options({ wasm, stateFile, prewarmedFiles: [], ...extra }),
    app.root,
  )
  const extract = (filename: string, code: string) =>
    core.extract({ filename, code, resourcePath: app.write(filename, code) })
  return { app, core, wasm, stateFile, extract }
}

afterEach(removeTestApps)

describe('sealed candidate boundaries', () => {
  it('restores the frozen theme after sheet import', async () => {
    // Given: theme-bearing CSS and no output cache.
    const theme = { colors: { default: { primary: 'red' } } }
    const f = fixture({
      cacheMaxEntries: 0,
      configureWasm: (wasm) => wasm.registerTheme(theme),
    })
    await f.extract('src/a.tsx', box('$primary'))
    const before = await f.core.css(query)
    expect(before.css).toContain('--primary')
    f.app.write(
      'devup.json',
      JSON.stringify({ theme: { colors: { default: { primary: 'blue' } } } }),
    )

    // When: production re-extracts against its frozen configuration, not new disk config.
    await f.extract('src/a.tsx', `${box('$primary')} // touched`)

    // Then: theme CSS remains exactly the served version.
    expect(await f.core.css(query)).toEqual(before)
  })

  it.each(['alias', 'restore mismatch', 'factory throw'])(
    'rejects invalid candidate factory: %s',
    async (kind) => {
      // Given: a factory that cannot provide an equivalent isolated engine.
      const app = createTestApp()
      const wasm = app.engine()
      const f = fixture({
        wasm,
        createEngine: () => {
          switch (kind) {
            case 'alias':
              return wasm
            case 'restore mismatch': {
              const engine = app.engine()
              return { ...engine, importSheet: () => undefined }
            }
            case 'factory throw':
              throw new Error('factory unavailable')
            default:
              throw new Error(`Unexpected fixture ${kind}`)
          }
        },
      })
      await f.extract('src/a.tsx', box('red'))
      const before = await f.core.css({ ...query, fileNum: 0 })
      const state = readFileSync(f.stateFile, 'utf8')

      // When: a cache miss needs a candidate.
      await expect(
        f.extract('src/a.tsx', `${box('red')} // touch`),
      ).rejects.toThrow(/^src\/a\.tsx:1:1:/)

      // Then: neither live sheet nor checkpoint changes.
      expect(wasm.getCss(0, false)).toBe(before.css)
      expect(readFileSync(f.stateFile, 'utf8')).toBe(state)
    },
  )

  it('allows existing shared-bucket atoms without a CSS revision', async () => {
    // Given: both sources belong to the same served private bucket.
    const canonicalMap = { 'src/a.tsx': 'src/a.tsx', 'src/b.tsx': 'src/a.tsx' }
    const f = fixture({
      canonicalMap,
      configureWasm: (wasm) => wasm.importCanonicalMap(canonicalMap),
    })
    await f.extract('src/a.tsx', box('red'))
    await f.extract('src/b.tsx', 'export const b = 1')
    const before = await f.core.css({ ...query, fileNum: 0 })
    const revision = readCoordinatorState(f.stateFile, '')?.revision

    // When: the sibling reuses an atom already in that bucket.
    const result = await f.extract('src/b.tsx', box('red'))

    // Then: its emitted selectors already exist and no CSS update is published.
    expect(result.updatedBaseStyle).toBe(false)
    expect(await f.core.css({ ...query, fileNum: 0 })).toEqual(before)
    expect(readCoordinatorState(f.stateFile, '')?.revision).toBe(revision)
  })

  it('rejects an existing value in a new private bucket', async () => {
    // Given: the atom exists only in bucket zero, not base or an unknown bucket.
    const f = fixture()
    await f.extract('src/a.tsx', box('red'))
    await f.core.css({ ...query, fileNum: 0 })
    const state = readFileSync(f.stateFile, 'utf8')

    // When: a new bucket wants the same value under its own missing selector.
    await expect(f.extract('src/new.tsx', box('red'))).rejects.toThrow(
      'change styles after the production stylesheet was served',
    )

    // Then: no new number or selector becomes live.
    expect(readFileSync(f.stateFile, 'utf8')).toBe(state)
    expect(f.wasm.exportFileMap()).toBe('{"src/a.tsx":0}')
  })

  it('retains historical atoms, keyframes and allocator tombstones without replay', async () => {
    // Given: retained CSS that the latest ledger source no longer describes.
    const f = fixture({ cacheMaxEntries: 0, sourceMap: false })
    f.wasm.importFileMap({ 'src/deleted.tsx': 7 })
    await f.extract(
      'src/a.tsx',
      `${box('red')} import { keyframes } from '@devup-ui/react'; export const spin = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } });`,
    )
    await f.extract('src/a.tsx', 'export const a = 1')
    const maps = readCoordinatorState(f.stateFile, '')
    const fileNum = Number(maps?.fileMap['src/a.tsx'])
    const before = await f.core.css({ ...query, fileNum, importMainCss: true })
    expect(before.css).toContain('@keyframes')
    expect(before.css).toContain('color:red')

    // When: an evicted source is re-extracted transactionally.
    await f.extract('src/a.tsx', 'export const a = 1 // touched')

    // Then: retained historical CSS and numbering survive unchanged.
    expect(
      await f.core.css({ ...query, fileNum, importMainCss: true }),
    ).toEqual(before)
    expect(readCoordinatorState(f.stateFile, '')?.classMap).toEqual(
      maps?.classMap,
    )
    expect(readCoordinatorState(f.stateFile, '')?.fileMap).toEqual(
      maps?.fileMap,
    )
  })

  it('serializes seal and rendering behind an already queued extraction', async () => {
    // Given: a complete plan, before any stylesheet is served.
    const f = fixture()
    await f.extract('src/a.tsx', box('red'))

    // When: extraction queues before the first CSS request.
    const extracting = f.extract('src/late.tsx', box('blue'))
    const serving = f.core.css({ ...query, fileNum: 1 })
    await extracting

    // Then: the producer completes before sealing, rather than being rejected mid-queue.
    expect((await serving).css).toContain('color:blue')
  })

  it.each(['red', 'blue'])(
    'invalidates dependency cache when imported color remains/becomes %s',
    async (color) => {
      // Given: a dependency-bearing extraction and its cached output.
      const f = fixture()
      f.app.write('src/tokens.ts', "export const color = 'red'")
      const source =
        "import { Box } from '@devup-ui/react'; import { color } from './tokens'; export const C = () => <Box color={color} />;"
      const first = await f.extract('src/a.tsx', source)
      expect(first.dependencies).toEqual(['src/tokens.ts'])
      const before = await f.core.css({ ...query, fileNum: 0 })
      f.app.write('src/tokens.ts', `export const color = '${color}' // touched`)

      // When: the same request must re-evaluate changed dependency bytes.
      const extracting = f.extract('src/a.tsx', source)

      // Then: exact CSS equality, rather than cache identity, decides acceptance.
      if (color === 'red') {
        expect((await extracting).updatedBaseStyle).toBe(false)
        expect(await f.core.css({ ...query, fileNum: 0 })).toEqual(before)
      } else {
        await expect(extracting).rejects.toThrow(
          'change styles after the production stylesheet was served',
        )
      }
    },
  )
})
