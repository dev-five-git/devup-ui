import { readFile, rm } from 'node:fs/promises'

import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { createBuilder, type UserConfig } from 'vite'

import { DevupUI } from '../plugin'
import { extractOwned, productionFixture } from './production-fixture'

it.each(['operation', 'native-input'])(
  'starts a fresh generation after %s failure (UNIT)',
  async (failure) => {
    // Given previously successful extraction and additional failed-interval state.
    const fixture = await productionFixture()
    await fixture.file('app/src/main.js', 'export const value=1;')
    const activation = await fixture.activation()
    await activation.prepare(() => {})
    const graph = { getModuleIds: () => [], getModuleInfo: () => null }
    const key = activation.context
    try {
      activation.start(key)
      activation.run(key, () => extractOwned('blue', 'previous.tsx'))
      await activation.finish(key, graph)
      activation.close(key)
      const before = await fixture.learned()
      const modules = activation.modules(key)
      activation.start(key)
      activation.run(key, () => extractOwned('green', 'failed.tsx'))
      if (failure === 'operation')
        expect(() =>
          activation.run(key, () => {
            throw new TypeError('input fault')
          }),
        ).toThrow(TypeError)
      await activation.finish(key, graph, new TypeError('native input fault'))
      expect(await fixture.learned()).toEqual(before)
      await expect(activation.prepare(() => {})).rejects.toMatchObject({
        name: 'ClosedBuildGenerationError',
      })
      // When the next native input boundary prepares a fresh owner.
      await activation.prepare(
        () => {},
        () => {},
      )
      activation.start(activation.context)
      const recovered = activation.run(activation.context, () =>
        extractOwned('red', 'recovered.tsx'),
      )
      // Then neither prior successful nor partial failed snapshots are restored.
      expect(activation.modules(activation.context)).not.toBe(modules)
      expect(recovered.state.files).not.toContain('previous.tsx')
      expect(recovered.state.files).not.toContain('failed.tsx')
      expect(recovered.css).toContain('background:red')
      activation.dispose()
      await expect(
        activation.prepare(
          () => {},
          () => {},
        ),
      ).rejects.toMatchObject({ name: 'ClosedBuildGenerationError' })
    } finally {
      activation.dispose()
      await fixture.close()
    }
  },
)

it.each([false, true])(
  'materializes only the fresh root when the SAME tuple is reused with singleCss=%s',
  async (singleCss) => {
    // Given a completed ordinary native A build and a different fresh root B.
    const first = await productionFixture()
    const second = await productionFixture()
    const tuple = DevupUI({ singleCss })
    await first.file(
      'app/src/main.js',
      "import {css} from '@devup-ui/react'; export const cls=css({background:'red'});",
    )
    await second.file(
      'app/src/main.js',
      "import {css} from '@devup-ui/react'; export const cls=css({background:'blue'});",
    )
    try {
      const a = await first.run(singleCss, [], { plugins: [tuple] })
      expect(
        a
          .filter((output) => output.type === 'asset')
          .map((output) => output.source)
          .join(''),
      ).toContain('background:red')
      const before = await first.learned()
      // When the public native build API resolves and builds B using those exact plugin objects.
      const b = await second.run(singleCss, [], { plugins: [tuple] })
      // Then B has its own linked CSS, materialization and ID-only stores, with no A bleed.
      const css = b
        .filter((output) => output.type === 'asset')
        .map((output) => output.source)
        .join('')
      const js = b
        .filter((output) => output.type === 'chunk')
        .map((output) => output.code)
        .join('')
      console.info(
        JSON.stringify({ case: 'same-tuple-fresh-root', singleCss, css, js }),
      )
      const rule =
        /\.([\w-]+)\s*\{[^}]*background\s*:\s*(?:blue|#00f|#0000ff)(?=[;}\s])/.exec(
          css,
        )
      expect(rule?.[1]).toBeDefined()
      expect(js).toContain(rule?.[1] ?? 'missing-class')
      expect(css).not.toContain('background:red')
      expect(js).not.toContain('@devup-ui/react')
      await readFile(`${second.root}/df/theme.d.ts`)
      await readFile(`${second.root}/df/compat.d.ts`)
      expect(
        (await second.learned()).every((id) => !id.includes(first.root)),
      ).toBe(true)
      expect(await first.learned()).toEqual(before)
      const files: unknown = JSON.parse(wasm.exportFileMap())
      expect(files).toEqual(
        expect.objectContaining({
          [`${second.root}/src/main.js`]: expect.any(Number),
        }),
      )
      expect(JSON.stringify(files)).not.toContain(first.root)
    } finally {
      await first.close()
      await second.close()
    }
  },
  60000,
)

it.each([false, true])(
  'keeps watch numbering after a native bundle closes with singleCss=%s',
  async (singleCss) => {
    // Given an actual default watch build with a scan-missed opaque styling source.
    const fixture = await productionFixture()
    try {
      // When native watch rebuilds a changed unrelated branch after the first bundle closes.
      const { first, second, ids } = await fixture.watchGeneration(singleCss)
      console.info(
        JSON.stringify({ case: 'same-tuple-watch', singleCss, first, second }),
      )
      // Then the original append-only map and stylesheet survive without fresh-generation reseeding.
      expect(ids).toContain('\0native:watch-style.js')
      expect(first?.css).toContain('background:red')
      expect(second?.files).toBe(first?.files)
      expect(second?.css).toBe(first?.css)
      expect(second?.code).toMatch(/value\s*=\s*2/)
    } finally {
      await fixture.close()
    }
  },
  60000,
)

it.each([false, true])(
  'rematerializes a clean same-root native generation with the SAME config and tuple with singleCss=%s',
  async (singleCss) => {
    // Given a completed build whose generated directory is removed by its owning fixture.
    const fixture = await productionFixture()
    const tuple = DevupUI({ singleCss })
    const options: UserConfig = { plugins: [tuple] }
    await fixture.file(
      'app/src/main.js',
      "import {css} from '@devup-ui/react'; export const cls=css({background:'red'});",
    )
    try {
      const first = await fixture.run(singleCss, [], options)
      const learned = await fixture.learned()
      await rm(`${fixture.root}/df`, { recursive: true })
      // When another ordinary native build resolves the same inputs and plugin tuple.
      const second = await fixture.run(singleCss, [], options)
      // Then actual native output is unchanged and all generated files are recreated.
      expect(
        second.map((output) =>
          output.type === 'asset' ? output.source : output.code,
        ),
      ).toEqual(
        first.map((output) =>
          output.type === 'asset' ? output.source : output.code,
        ),
      )
      await readFile(`${fixture.root}/df/theme.d.ts`)
      await readFile(`${fixture.root}/df/compat.d.ts`)
      expect(await fixture.learned()).toEqual(learned)
    } finally {
      await fixture.close()
    }
  },
  60000,
)

it.each([false, true])(
  'retains the shared native builder union across sibling closes with singleCss=%s',
  async (singleCss) => {
    // Given separately resolved native sibling configurations sharing the SAME tuple.
    const fixture = await productionFixture()
    const first = await fixture.file(
      'app/src/main.js',
      "import {css} from '@devup-ui/react'; export const cls=css({background:'red'});",
    )
    const second = await fixture.file(
      'app/src/sibling.js',
      "import {css} from '@devup-ui/react'; export const cls=css({background:'blue'});",
    )
    const tuple = DevupUI({ singleCss })
    const maps = new Map<string, string>()
    try {
      const builder = await createBuilder({
        root: fixture.root,
        configFile: false,
        logLevel: 'silent',
        plugins: [
          tuple,
          {
            name: 'observe-shared-native-map',
            generateBundle() {
              maps.set(this.environment.name, wasm.exportFileMap())
            },
          },
        ],
        environments: {
          client: { build: { lib: { entry: first, formats: ['es'] } } },
          sibling: {
            consumer: 'client',
            build: { lib: { entry: second, formats: ['es'] } },
          },
        },
        build: { write: false, lib: { entry: first, formats: ['es'] } },
        builder: {
          async buildApp(builder) {
            const { client, sibling } = builder.environments
            if (client === undefined || sibling === undefined)
              throw new TypeError('Expected native siblings')
            await builder.build(client)
            await builder.build(sibling)
          },
        },
      })
      // When public buildApp builds both siblings sequentially, closing the first before the second.
      await builder.buildApp()
      // Then both completed graphs retain the same union numbers, never a per-sibling reset.
      expect(maps.size).toBe(2)
      expect(maps.get('sibling')).toBe(maps.get('client'))
      for (const map of maps.values()) {
        const files: unknown = JSON.parse(map)
        expect(files).toEqual(
          expect.objectContaining({
            [first]: expect.any(Number),
            [second]: expect.any(Number),
          }),
        )
      }
    } finally {
      await fixture.close()
    }
  },
  60000,
)
