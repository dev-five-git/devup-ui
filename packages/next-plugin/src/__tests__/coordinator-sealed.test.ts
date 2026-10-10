import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import type { CoordinatorOptions } from '../coordinator-options'
import { readCoordinatorState } from '../state'
import { createTestApp, removeTestApps } from './coordinator-app'

const global = (color: string) =>
  `import { globalCss } from '@devup-ui/react'; globalCss({ body: { color: '${color}' } });`
const box = (color: string) =>
  `import { Box } from '@devup-ui/react'; export const C = () => <Box color="${color}" />;`
const query = { wait: true, importMainCss: false }

async function fixture(extra: Partial<CoordinatorOptions> = {}) {
  const app = createTestApp()
  const stateFile = join(app.root, 'df/state.json')
  const revisionFile = join(app.root, 'df/revision')
  const wasm = app.engine()
  const core = createCore(
    app.options({
      wasm,
      stateFile,
      revisionFile,
      prewarmedFiles: [],
      ...extra,
    }),
    app.root,
  )
  await core.startup()
  const extract = (filename: string, code: string) =>
    core.extract({ filename, code, resourcePath: app.write(filename, code) })
  return { app, core, wasm, extract, stateFile, revisionFile }
}

afterEach(removeTestApps)

describe('transactional sealed extraction with real WASM', () => {
  it.each([0, 1])(
    'accepts touched global CSS after cache eviction at capacity %i',
    async (cacheMaxEntries) => {
      // Given: a served sheet and an evicted output.
      const f = await fixture({ cacheMaxEntries })
      await f.extract('src/a.tsx', global('red'))
      await f.extract('src/b.tsx', 'export const b = 1')
      const before = await f.core.css(query)
      const revision = readFileSync(f.revisionFile, 'utf8')

      // When: the source is touched without changing its global style.
      const result = await f.extract('src/a.tsx', `${global('red')} // touched`)

      // Then: the rendered sheet and CSS revision are unchanged.
      expect(result.updatedBaseStyle).toBe(false)
      expect(await f.core.css(query)).toEqual(before)
      expect(readFileSync(f.revisionFile, 'utf8')).toBe(revision)
      expect(
        readCoordinatorState(f.stateFile, '')?.inputs.find(
          (input) => input.filename === 'src/a.tsx',
        )?.source,
      ).toEndWith('// touched')
    },
  )

  it.each([
    ['ordinary atom', box('tan')],
    ['global same-length value', global('tan')],
    [
      'import',
      "import { globalCss } from '@devup-ui/react'; globalCss({ imports: ['https://example.test/new.css'] });",
    ],
    [
      'font face',
      "import { globalCss } from '@devup-ui/react'; globalCss({ fontFaces: [{ fontFamily: 'New', src: 'local(New)' }] });",
    ],
  ])('rejects changed %s without altering live state', async (_kind, code) => {
    // Given: live CSS, allocator maps, ledger and durable revision.
    const f = await fixture()
    await f.extract('src/a.tsx', `${box('red')} ${global('red')}`)
    const before = await f.core.css(query)
    const state = readFileSync(f.stateFile, 'utf8')
    const maps = [
      f.wasm.exportSheet(),
      f.wasm.exportClassMap(),
      f.wasm.exportFileMap(),
    ]
    const revision = readFileSync(f.revisionFile, 'utf8')

    // When: a candidate introduces CSS absent from the served sheet.
    await expect(f.extract('src/a.tsx', code)).rejects.toThrow(
      /^src\/a\.tsx:1:1:.*Fix: include it in expectedBaseFiles/,
    )

    // Then: rejection preserves every live and durable state component.
    expect([
      f.wasm.exportSheet(),
      f.wasm.exportClassMap(),
      f.wasm.exportFileMap(),
    ]).toEqual(maps)
    expect(readFileSync(f.stateFile, 'utf8')).toBe(state)
    expect(readFileSync(f.revisionFile, 'utf8')).toBe(revision)
    await f.extract('src/a.tsx', `${box('red')} ${global('red')}`)
    expect(await f.core.css(query)).toEqual(before)
  })

  it('accepts evicted byte-identical global source', async () => {
    // Given: no cached output survives the initial extraction.
    const f = await fixture({ cacheMaxEntries: 0 })
    await f.extract('src/a.tsx', global('red'))
    const before = await f.core.css(query)

    // When: the exact same source must be extracted again.
    const result = await f.extract('src/a.tsx', global('red'))

    // Then: an engine dirty flag does not imply a CSS change.
    expect(result.updatedBaseStyle).toBe(false)
    expect(await f.core.css(query)).toEqual(before)
  })

  it.each([
    ['imports', "imports: ['https://example.test/new.css']"],
    ['fontFaces', "fontFaces: [{ fontFamily: 'New', src: 'local(New)' }]"],
  ])(
    'rejects new %s even without ordinary-atom output',
    async (_kind, rules) => {
      // Given: an ordinary private atom and no existing global CSS.
      const f = await fixture()
      await f.extract('src/a.tsx', box('red'))
      const before = await f.core.css(query)
      const state = readFileSync(f.stateFile, 'utf8')

      // When: only an import or font face changes the rendered sheet.
      await expect(
        f.extract(
          'src/new.tsx',
          `import { globalCss } from '@devup-ui/react'; globalCss({ ${rules} });`,
        ),
      ).rejects.toThrow(
        'change styles after the production stylesheet was served',
      )

      // Then: rendered CSS, maps and checkpoint still describe the served sheet.
      expect(await f.core.css(query)).toEqual(before)
      expect(readFileSync(f.stateFile, 'utf8')).toBe(state)
    },
  )

  it('compares global CSS content rather than equal string lengths', async () => {
    // Given: red and tan render equally long but different global styles.
    const f = await fixture()
    await f.extract('src/a.tsx', global('red'))
    const before = await f.core.css(query)
    const probe = f.app.engine()
    probe
      .codeExtract(
        'src/a.tsx',
        global('tan'),
        '@devup-ui/react',
        './df',
        false,
        false,
        true,
        {},
      )
      .free()
    expect(probe.getCss(undefined, false).length).toBe(before.css.length)
    expect(probe.getCss(undefined, false)).not.toBe(before.css)

    // When: a same-length replacement attempts to cross the seal.
    await expect(f.extract('src/a.tsx', global('tan'))).rejects.toThrow(
      'change styles after the production stylesheet was served',
    )

    // Then: the served value remains red.
    expect(f.wasm.getCss(undefined, false)).toBe(before.css)
  })

  it('discards a throwing candidate even after it allocates names and CSS', async () => {
    // Given: the factory supplies a real engine whose extraction throws after mutation.
    const app = createTestApp()
    const f = await fixture({
      createEngine: () => {
        const engine = app.engine()
        return {
          ...engine,
          codeExtract(...args) {
            engine.codeExtract(...args).free()
            throw new Error('candidate failed after allocation')
          },
        }
      },
    })
    await f.extract('src/a.tsx', box('red'))
    const before = await f.core.css(query)
    const state = readFileSync(f.stateFile, 'utf8')

    // When: extraction fails after having mutated the isolated candidate.
    await expect(f.extract('src/new.tsx', box('blue'))).rejects.toThrow(
      'src/new.tsx:1:1:',
    )

    // Then: neither live CSS nor its checkpoint incorporates that allocation.
    expect(await f.core.css(query)).toEqual(before)
    expect(readFileSync(f.stateFile, 'utf8')).toBe(state)
  })
})
