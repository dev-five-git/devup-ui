import { readFileSync, rmSync } from 'node:fs'
import { join } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import {
  type CoordinatorInstance,
  createInstance,
} from '../coordinator-instance'
import { createInput, stampFile } from '../coordinator-ledger'
import type { PreparedSourceGeneration } from '../coordinator-options'
import { readCoordinatorState } from '../state'
import {
  connect,
  createTestApp,
  failure,
  removeTestApps,
} from './coordinator-app'
import {
  box,
  compiledSource,
  cssQuery,
  generation,
  preparedCore,
  prewarmed,
} from './coordinator-prepared-fixture'

const instances: CoordinatorInstance[] = []
afterEach(() => {
  for (const instance of instances.splice(0)) instance.close()
  removeTestApps()
})

it.each(['css', 'theme', 'delete', 'watch', 'extract'] as const)(
  'refreshes only dirty compiled inputs before %s work, without native MDX resend',
  async (operation) => {
    // Given controlled compiler outputs for two MDX files and accepted ordinary sources.
    const app = createTestApp()
    app.write('src/page.mdx', '# initial raw')
    app.write('src/other.mdx', '# untouched raw')
    const dependency = app.write('compiler-dependency.txt', 'initial')
    const theme = app.write(
      'devup.json',
      JSON.stringify({ theme: { colors: { default: { primary: '#f00' } } } }),
    )
    const gone = app.write('src/gone.tsx', box('teal'))
    const ordinary = ['gone', 'keeper'].map((name) =>
      createInput(
        app.root,
        {
          filename: `src/${name}.tsx`,
          resourcePath: join(app.root, `src/${name}.tsx`),
          code: box(name === 'gone' ? 'teal' : 'purple'),
        },
        [],
      ),
    )
    const page = compiledSource(app, {
      filename: 'src/page.mdx',
      code: box('red'),
      dependencies: [dependency],
    })
    const other = compiledSource(app, {
      filename: 'src/other.mdx',
      code: box('pink'),
    })
    let staged = page
    const refreshed: string[] = []
    const prepared = prewarmed(app, {
      generation: generation([page, other]),
      ordinaryInputs: ordinary,
      prepareReplay: async ({ generation: current }) => {
        const dirty = current.sources.filter(({ evidence }) =>
          Object.entries(evidence.fileFingerprints).some(
            ([path, stamp]) => stampFile(path) !== stamp,
          ),
        )
        if (dirty.length === 0) return current
        refreshed.push(...dirty.map(({ input }) => input.filename))
        return generation(
          current.sources.map((source) =>
            source.input.filename === staged.input.filename ? staged : source,
          ),
        )
      },
    })
    const core = createCore({ ...prepared.options, devupFile: theme }, app.root)
    await core.startup()
    app.write('src/page.mdx', '# edited raw')
    app.write('compiler-dependency.txt', 'edited')
    staged = compiledSource(app, {
      filename: 'src/page.mdx',
      code: box('green'),
      dependencies: [dependency],
    })
    if (operation === 'theme')
      app.write(
        'devup.json',
        JSON.stringify({ theme: { colors: { default: { primary: '#00f' } } } }),
      )
    if (operation === 'delete') rmSync(gone)

    // When CSS/extraction/watcher reconciliation is requested before the native loader resends MDX.
    if (operation === 'watch') await core.reconcile()
    else if (operation === 'extract')
      await core.extract({
        filename: 'src/new.tsx',
        resourcePath: join(app.root, 'src/new.tsx'),
        code: box('orange'),
      })
    else await core.css({ ...cssQuery, fileNum: 3 })

    // Then replay uses the fresh bytes, preserves unrelated inputs, and commits one refresh revision.
    const snapshot = readCoordinatorState(prepared.options.stateFile ?? '', '')
    expect(
      snapshot?.inputs.find((input) => input.filename === 'src/page.mdx')
        ?.source,
    ).toBe(box('green'))
    expect(
      snapshot?.inputs.find((input) => input.filename === 'src/other.mdx')
        ?.source,
    ).toBe(box('pink'))
    expect(
      snapshot?.inputs.find((input) => input.filename === 'src/keeper.tsx')
        ?.source,
    ).toBe(box('purple'))
    expect((await core.css({ ...cssQuery, fileNum: 3 })).css).toContain(
      'background:green',
    )
    expect(refreshed).toEqual(['src/page.mdx'])
    expect(snapshot?.revision).toBe(operation === 'extract' ? 9 : 8)
    if (operation === 'theme')
      expect(
        (await core.css({ ...cssQuery, fileNum: undefined })).css,
      ).toContain('--primary:#00F')
    if (operation === 'delete')
      expect(
        snapshot?.inputs.some((input) => input.filename === 'src/gone.tsx'),
      ).toBe(false)
    core.close()
  },
)

it('blocks a native resend from an older compiled generation instead of replacing fresh replay', async () => {
  // Given a refreshed durable generation while the native loader still holds old bytes.
  const app = createTestApp()
  app.write('src/page.mdx', '# raw')
  const staged: { generation?: PreparedSourceGeneration } = {}
  const prepared = await preparedCore(app, {
    generation: generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('red') }),
    ]),
    prepareReplay: async ({ generation }) =>
      generation.sources[0]?.input.source === box('green')
        ? generation
        : (staged.generation ?? generation),
  })
  staged.generation = generation([
    compiledSource(app, { filename: 'src/page.mdx', code: box('green') }),
  ])
  await prepared.core.css(cssQuery)
  const before = readFileSync(prepared.options.stateFile ?? '', 'utf8')

  // When a late native request sends the previously accepted bytes.
  const error = await failure(
    prepared.core.extract({
      filename: 'src/page.mdx',
      resourcePath: join(app.root, 'src/page.mdx'),
      code: box('red'),
    }),
  )

  // Then stale bytes are rejected and the fresh durable input survives.
  expect(String(error)).toContain('src/page.mdx:1:1:')
  expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(before)
  expect((await prepared.core.css(cssQuery)).css).toContain('background:green')
  prepared.core.close()
})

it('refreshes dirty MDX through a real source watcher without any CSS or extract request', async () => {
  // Given a running HTTP coordinator whose watcher will enter a controlled provider.
  const app = createTestApp()
  app.write('src/page.mdx', '# old')
  const entered = Promise.withResolvers<void>()
  const old = compiledSource(app, {
    filename: 'src/page.mdx',
    code: box('red'),
  })
  const staged: { generation?: PreparedSourceGeneration } = {}
  const prepared = prewarmed(app, {
    generation: generation([old]),
    prepareReplay: async ({ generation }) => {
      entered.resolve()
      return generation.sources[0]?.input.source === box('green')
        ? generation
        : (staged.generation ?? generation)
    },
  })
  const instance = createInstance({
    ...prepared.options,
    sourceRoots: [join(app.root, 'src')],
  })
  instances.push(instance)
  await instance.ready
  staged.generation = generation([
    compiledSource(app, { filename: 'src/page.mdx', code: box('green') }),
  ])

  // When only a source filesystem event occurs.
  app.write('src/page.mdx', '# new')
  await entered.promise
  await instance.flush()

  // Then the watcher alone committed fresh compiled bytes before a native resend.
  expect(
    readCoordinatorState(prepared.options.stateFile ?? '', '')?.inputs[0]
      ?.source,
  ).toBe(box('green'))
  expect(
    (await connect(app.portFile, app.identity).get('/css?fileNum=0')).body,
  ).toContain('background:green')
  await instance.drain()
})
