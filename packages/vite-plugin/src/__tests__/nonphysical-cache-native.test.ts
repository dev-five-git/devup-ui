import { readFile, writeFile } from 'node:fs/promises'

import { BuildGeneration, runBuildOperation } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'
import { build, type Plugin } from 'vite'

import { NonphysicalModules } from '../nonphysical-modules'
import { DevupUI } from '../plugin'
import { productionEngine, type ProductionState } from '../production-engine'
import {
  extractOwned,
  nativeCompletionDeadline,
  nativeCompletions,
  productionFixture,
} from './production-fixture'

type TestPlugin = Omit<Plugin, 'watchChange' | 'closeWatcher'> & {
  watchChange(this: void, id: string): Promise<void>
  closeWatcher(): void
}

it('rejects an unsettled completion with a real deadline error (UNIT)', async () => {
  // Given a completion with no native success/error signal.
  let timer: ReturnType<typeof setTimeout> | undefined
  const completion = new Promise<void>((_resolve, reject) => {
    timer = nativeCompletionDeadline(reject, 1)
  })
  try {
    // When the actual platform deadline fires.
    // Then completion rejects with the configured deadline error.
    await expect(completion).rejects.toBeInstanceOf(RangeError)
  } finally {
    clearTimeout(timer)
  }
})

it('updates the production sheet through the actual tuple theme hook (UNIT dispatch)', async () => {
  // Given a native-prepared tuple and a real theme file.
  const fixture = await productionFixture()
  const tuple = DevupUI() as unknown as [TestPlugin]
  await fixture.file('app/src/main.js', 'export const value=1;')
  const theme = await fixture.file(
    'app/devup.json',
    '{"theme":{"colors":{"default":{"primary":"red"}}}}',
  )
  try {
    await fixture.run(false, [], { plugins: [tuple] })
    await writeFile(
      theme,
      '{"theme":{"colors":{"default":{"primary":"blue"}}}}',
    )
    // When the public hook dispatches the replacement theme for that same generation.
    await tuple[0].watchChange.call(undefined, theme)
    // Then the actual materialized production sheet contains the new theme.
    const css = await readFile(
      `${fixture.root}/df/devup-ui/devup-ui.css`,
      'utf8',
    )
    expect(css).toContain('--primary:blue')
    expect(css).not.toContain('--primary:red')
  } finally {
    tuple[0].closeWatcher()
    await fixture.close()
  }
})

it('restores an owned Vite sheet after a sibling operates when only its own legacy interval ends', () => {
  // Given two real snapshot owners after the actual Vite factory releases its interval.
  const [plugin] = DevupUI() as unknown as [{ closeBundle(this: void): void }]
  plugin.closeBundle.call(undefined)
  const first = new BuildGeneration<ProductionState>()
  const sibling = new BuildGeneration<ProductionState>()
  const context = { integration: 'Vite', root: process.cwd() }
  let admitted = false
  function run<T>(owner: BuildGeneration<ProductionState>, action: () => T): T {
    return runBuildOperation(context, () => {
      admitted = true
      return owner.run(productionEngine, action)
    })
  }
  try {
    const original = run(first, () => extractOwned('red'))
    run(sibling, () => extractOwned('blue'))
    // When the first owner resumes through the real admitted reset/restore boundary.
    const restored = run(first, () => ({
      state: productionEngine.capture(),
      css: wasm.getCss(0, false),
    }))
    // Then all four actual snapshots and its finished red sheet are restored unchanged.
    expect(restored.state).toEqual(original.state)
    expect(restored.css).toBe(original.css)
    expect(restored.css).toContain('background:red')
  } finally {
    first.dispose()
    sibling.dispose()
    if (admitted) runBuildOperation(context, () => wasm.resetBuildState())
  }
})

it.each([false, true])(
  'observes opaque extraction in every completed native interval with singleCss=%s',
  async (singleCss) => {
    // Given a real watch graph with a styled opaque module and an unrelated source.
    const fixture = await productionFixture()
    const opaque = '\0native:cached-style.js'
    const recoveredId = '\0native:recovered-style.js'
    const buildOptions = {
      write: true,
      rolldownOptions: {
        external: ['react/jsx-runtime', 'react/jsx-dev-runtime'],
      },
    }
    await fixture.file(
      'app/index.html',
      '<script type="module" src="/src/main.js"></script>',
    )
    await fixture.file(
      'app/src/main.js',
      "import {cls} from 'virtual-style'; import {value} from './unrelated.jsx'; globalThis.nativeStyle=cls; globalThis.nativeValue=value;",
    )
    const unrelated = await fixture.file(
      'app/src/unrelated.jsx',
      'export const value=1;',
    )
    const cohorts: string[][] = []
    const finish = NonphysicalModules.prototype.finish
    const observed: string[][] = []
    const inputCodes: string[] = []
    const nativeCodes: (string | null)[] = []
    const outputs: (() => Promise<
      { readonly name: string; readonly bytes: Buffer }[]
    >)[] = []
    const provider: Plugin = {
      name: 'actual-native-cache-provider',
      resolveId(id) {
        if (id === 'virtual-style') return opaque
        if (id === 'virtual-recovered') return recoveredId
      },
      load(id) {
        if (id === opaque || id === recoveredId)
          return "import {css} from '@devup-ui/react'; export const cls=css({color:'red'});"
      },
      writeBundle(_options, bundle) {
        const names = Object.keys(bundle)
          .filter((name) => /\.(js|css)$/.test(name))
          .sort()
        outputs.push(async () =>
          Promise.all(
            names.map(async (name) => ({
              name,
              bytes: await readFile(`${fixture.root}/dist/${name}`),
            })),
          ),
        )
      },
    }
    const spy = spyOn(
      NonphysicalModules.prototype,
      'finish',
    ).mockImplementation(function (this: NonphysicalModules, graph, learned) {
      observed.push([...this.observations()])
      inputCodes.push(graph.getModuleInfo(unrelated)?.code ?? '')
      nativeCodes.push(graph.getModuleInfo(opaque)?.code ?? null)
      return finish.call(this, graph, learned).then((result) => {
        cohorts.push([...result.ids])
        return result
      })
    })
    let close: (() => Promise<void>) | undefined
    try {
      const result = await build({
        root: fixture.root,
        configFile: false,
        logLevel: 'silent',
        plugins: [DevupUI({ singleCss }), provider],
        build: {
          ...buildOptions,
          watch: {},
        },
      })
      if (Array.isArray(result) || !('on' in result))
        throw new TypeError('Expected native watcher')
      close = () => result.close()
      const next = nativeCompletions(result)
      await next()
      expect(observed[0]).toContain(opaque)
      expect(await fixture.learned()).toContain(opaque)
      // When default native watch rebuilds an unrelated branch.
      await writeFile(unrelated, 'export const value=2;')
      await next()
      while (!inputCodes.some((code) => /value\s*=\s*2/.test(code)))
        await next()
      const changed = inputCodes.findIndex((code) => /value\s*=\s*2/.test(code))
      console.info(
        JSON.stringify({
          case: 'native-cache-observation',
          singleCss,
          nativeCodes,
          inputCodes,
          observations: observed,
          completedCohorts: cohorts,
          changed,
        }),
      )
      expect(typeof nativeCodes[changed]).toBe('string')
      // Then Vite re-extracts the opaque module in every successfully completed input interval.
      expect(cohorts.length).toBeGreaterThan(1)
      expect(observed).toHaveLength(cohorts.length)
      for (const [index, cohort] of cohorts.entries()) {
        expect(observed[index]).toContain(opaque)
        expect(typeof nativeCodes[index]).toBe('string')
        expect(cohort).toContain(opaque)
      }
      expect(await fixture.learned()).toContain(opaque)
      const before = await fixture.storeBytes()
      await writeFile(
        unrelated,
        "import {Box} from '@devup-ui/react'; export const value=<Box",
      )
      await expect(next()).rejects.toThrow()
      expect(await fixture.storeBytes()).toEqual(before)
      // When malformed physical JSX becomes valid in this SAME native watcher.
      await writeFile(
        unrelated,
        "import 'virtual-recovered'; import {Box} from '@devup-ui/react'; export const value=<Box bg='red'/>;",
      )
      await next()
      const recoveredCapture = outputs.at(-1)
      if (!recoveredCapture)
        throw new TypeError('Expected recovered output capture')
      const recovered = await recoveredCapture()
      expect(await fixture.learned()).toContain(recoveredId)
      await close()
      close = undefined
      for (const [name, bytes] of Object.entries(before))
        await writeFile(`${fixture.root}/df/numbering/${name}`, bytes)
      const captureCount = outputs.length
      await fixture.run(singleCss, [provider], { build: buildOptions })
      const cleanCapture = outputs[captureCount]
      if (!cleanCapture)
        throw new TypeError('Expected new clean output capture')
      const clean = await cleanCapture()
      // Then raw JS/CSS equal a clean build with the same root/options/source/starting ID history.
      expect(recovered).toEqual(clean)
      const recoveredText = recovered
        .map(({ bytes }) => bytes.toString())
        .join('')
      expect(recoveredText).toMatch(/background\s*:\s*(red|#f00)/)
      expect(recoveredText).toContain('div')
      const rule = /\.([\w-]+)\s*\{background:(?:red|#f00)/.exec(recoveredText)
      expect(rule?.[1]).toBeDefined()
      expect(
        clean
          .filter(({ name }) => name.endsWith('.js'))
          .map(({ bytes }) => bytes.toString())
          .join(''),
      ).toContain(rule?.[1] ?? 'missing-class')
    } finally {
      await close?.()
      spy.mockRestore()
      await fixture.close()
    }
  },
  60000,
)
