import { execFileSync } from 'node:child_process'
import { chmod, unlink, writeFile } from 'node:fs/promises'

import { createNonphysicalIdStore } from '@devup-ui/plugin-utils'
import { exportFileMap } from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { type Plugin, resolveConfig } from 'vite'

import { NonphysicalModules } from '../nonphysical-modules'
import { DevupUI } from '../plugin'
import { ProductionActivation } from '../production-numbering'
import { productionFixture } from './production-fixture'

it('leaves a configured but unbuilt context store untouched during native completion', async () => {
  // Given an actual scoped sibling store and an ordinary client entry.
  const fixture = await productionFixture()
  const entry = await fixture.file('app/src/main.js', 'export const value=1;')
  const store = createNonphysicalIdStore({
    integration: 'Vite',
    resolvedRoot: fixture.root,
    contextKey: 'unbuilt',
    distDir: `${fixture.root}/df`,
  })
  await store.rewrite(['virtual:unbuilt'], [])
  try {
    // When only the client is built through the native API.
    await fixture.run(false, [], {
      environments: { client: {}, unbuilt: { consumer: 'server' } },
    })
    // Then the sibling is not rewritten as an empty or inferred cohort.
    expect(await store.read()).toEqual({
      kind: 'loaded',
      ids: ['virtual:unbuilt'],
    })
    const numbers: unknown = JSON.parse(exportFileMap())
    expect(numbers).toEqual({ [entry]: 0, 'virtual:unbuilt': 1 })
  } finally {
    await fixture.close()
  }
})

it('subtracts a learned physical arrival when a fresh native generation promotes it into SCAN', async () => {
  // Given a real first-arrival source outside the initially scanned parent.
  const fixture = await productionFixture()
  const physical = await fixture.file(
    'outside/view.js',
    "import {css} from '@devup-ui/react'; export const cls=css({color:'red'});",
  )
  await fixture.file('app/src/main.js', "export {cls} from 'outside-view';")
  const provider: Plugin = {
    name: 'promoted-arrival',
    resolveId(id) {
      if (id === 'outside-view') return physical
    },
  }
  try {
    await fixture.run(false, [provider])
    expect(await fixture.learned()).toContain(physical)
    // When the same actual source joins the next native generation's scan roots.
    await fixture.run(false, [], {
      plugins: [
        DevupUI({ sourceDirs: [`${fixture.directory}/outside`] }),
        provider,
      ],
    })
    // Then learned storage no longer duplicates the scan reservation.
    expect(await fixture.learned()).not.toContain(physical)
  } finally {
    await fixture.close()
  }
})

it('propagates a genuine physical retention IO fault rather than treating it as absence (UNIT)', async () => {
  // Given real OS permissions denying access outside the scanned application.
  const fixture = await productionFixture()
  const physical = await fixture.file('denied/source.js', 'export {}')
  const directory = `${fixture.directory}/denied`
  const account =
    process.platform === 'win32'
      ? execFileSync('whoami', { encoding: 'utf8' }).trim()
      : ''
  if (process.platform === 'win32')
    execFileSync('icacls', [directory, '/deny', `${account}:(OI)(CI)(F)`])
  else await chmod(directory, 0o000)
  const modules = new NonphysicalModules([])
  try {
    // When empty-O retention follows the exact saved physical spelling.
    const completion = modules.finish(
      {
        *getModuleIds() {},
        getModuleInfo: () => null,
      },
      [physical],
    )
    // Then the real filesystem failure is propagated, never converted to a missing file.
    await expect(completion).rejects.toBeInstanceOf(Error)
  } finally {
    if (process.platform === 'win32')
      execFileSync('icacls', [directory, '/remove:d', account, '/T', '/C'])
    else await chmod(directory, 0o700)
    await fixture.close()
  }
})

it('propagates genuine numbering-store IO failure before native extraction', async () => {
  // Given a real non-directory blocking the unchanged store path.
  const fixture = await productionFixture()
  await fixture.file('app/src/main.js', 'export const value=1;')
  await fixture.file('app/df/numbering', 'not a directory')
  try {
    // When ordinary native preparation materializes its scoped store.
    const operation = fixture.run(false)
    // Then the real native mkdir fault is propagated, not corrupt/missing history.
    await expect(operation).rejects.toThrow('EEXIST')
  } finally {
    await fixture.close()
  }
})

it('replays changed theme state when an already-prepared production generation updates its theme', async () => {
  // Given a real configuration and an allocation-inert prepared owner.
  const fixture = await productionFixture()
  const entry = await fixture.file('app/src/main.js', 'export const value=1;')
  const theme = await fixture.file(
    'app/devup.json',
    '{"theme":{"colors":{"default":{"primary":"red"}}}}',
  )
  const activation = new ProductionActivation(
    {
      package: '@devup-ui/react',
      devupFile: 'devup.json',
      distDir: 'df',
      cssDir: undefined,
      extractCss: true,
      debug: false,
      include: [],
      singleCss: false,
      prefix: undefined,
      shorthands: undefined,
      sourceDirs: undefined,
      mdxExtensions: ['.mdx'],
      atomHoist: undefined,
      importAliases: {},
    },
    () => {},
  )
  try {
    activation.configure(
      await resolveConfig(
        {
          root: fixture.root,
          configFile: false,
          logLevel: 'silent',
          build: { lib: { entry, formats: ['es'] } },
        },
        'build',
      ),
    )
    await activation.prepare(() => {})
    await writeFile(
      theme,
      '{"theme":{"colors":{"default":{"primary":"blue"}}}}',
    )
    // When the same prepared generation executes its real theme-update boundary.
    await activation.updateTheme()
    // Then the resumed native sheet contains the replacement theme, not the prior snapshot.
    expect(activation.css(null, false)).toContain('--primary:blue')
    expect(activation.css(null, false)).not.toContain('--primary:red')
  } finally {
    activation.dispose()
    await fixture.close()
  }
})

it.each([false, true])(
  'learns real native arrivals and retains only eligible history with singleCss=%s',
  async (singleCss) => {
    // Given sources outside the scanned parent and a native opaque producer.
    const fixture = await productionFixture()
    const physical = await fixture.file(
      'outside/view.js',
      "import {css} from '@devup-ui/react'; export const cls=css({color:'red'});",
    )
    const opaque = '\0native:v1-style.js'
    const provider: Plugin = {
      name: 'native-arrival-provider',
      resolveId(id) {
        if (id === 'outside-view') return physical
        if (id === 'virtual-view') return opaque
      },
      load(id) {
        if (id === opaque)
          return "import {css} from '@devup-ui/react'; export const cls=css({background:'blue'});"
      },
    }
    const entry = await fixture.file(
      'app/src/main.js',
      "export {cls as physical} from 'outside-view'; export {cls as virtual} from 'virtual-view';",
    )
    try {
      // When native resolution introduces both unreserved extraction sources.
      await fixture.run(singleCss, [provider])
      const first = await fixture.learned()
      expect(first).toContain(physical)
      expect(first).toContain(opaque)
      await fixture.run(singleCss, [provider])
      expect(await fixture.learned()).toEqual(first)
      await writeFile(entry, 'export const untouched=1;')
      await fixture.run(singleCss)
      // Then existing-but-unreachable physical history survives while unevidenced opaque history drops.
      expect(await fixture.learned()).toContain(physical)
      expect(await fixture.learned()).not.toContain(opaque)
      await unlink(physical)
      await fixture.run(singleCss)
      expect(await fixture.learned()).not.toContain(physical)
    } finally {
      await fixture.close()
    }
  },
  30000,
)

it.each([false, true])(
  'withholds failed input but commits completed input before a later output fault with singleCss=%s',
  async (singleCss) => {
    // Given a real opaque source and an entry with no styles of its own.
    const fixture = await productionFixture()
    await fixture.file('app/src/main.js', "export {cls} from 'virtual-view';")
    const opaque = '\0native:timing.js'
    const provider: Plugin = {
      name: 'timing-provider',
      resolveId(id) {
        if (id === 'virtual-view') return opaque
      },
      load(id) {
        if (id === opaque)
          return "import {css} from '@devup-ui/react'; export const cls=css({color:'red'});"
      },
    }
    try {
      // When extraction succeeds and native output fails only after buildEnd completion.
      await expect(
        fixture.run(singleCss, [
          provider,
          {
            name: 'late-native-output-fault',
            renderStart() {
              throw new RangeError('late output fault')
            },
          },
        ]),
      ).rejects.toThrow('late output fault')
      // Then the actual completed-input cohort is already committed.
      expect(await fixture.learned()).toContain(opaque)
      await fixture.file('app/src/main.js', "export {cls} from 'virtual-new';")
      const before = await fixture.learned()
      await expect(
        fixture.run(singleCss, [
          {
            name: 'native-input-fault',
            resolveId(id) {
              if (id === 'virtual-new') throw new RangeError('input fault')
            },
          },
        ]),
      ).rejects.toThrow('input fault')
      expect(await fixture.learned()).toEqual(before)
    } finally {
      await fixture.close()
    }
  },
  30000,
)
