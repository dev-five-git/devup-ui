import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { join } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import {
  type CoordinatorOptions,
  flushCoordinatorWrites,
  resetCoordinator,
  startCoordinator,
} from '../coordinator'
import { readCoordinatorState } from '../state'
import type { DevupWasm } from '../wasm'
import {
  connect,
  createTestApp,
  eventually,
  instrument,
  removeTestApps,
  type TestApp,
} from './coordinator-app'

const box = (bg: string) =>
  `import { Box } from '@devup-ui/react'\nexport const C = () => <Box bg="${bg}" p={4} />\n`
const globalBox = (bg: string, margin: number) =>
  `import { Box, globalCss } from '@devup-ui/react'\nglobalCss({ body: { margin: ${margin} } })\nexport const G = () => <Box bg="${bg}" />\n`

const stateOf = (app: TestApp) => join(app.root, 'df', 'state.json')
const revisionOf = (app: TestApp) => join(app.root, 'df', 'revision')
const revision = (app: TestApp) =>
  Number(readFileSync(revisionOf(app), 'utf-8'))

function dev(app: TestApp, extra: Partial<CoordinatorOptions> = {}) {
  return app.options({
    watch: true,
    stateFile: stateOf(app),
    revisionFile: revisionOf(app),
    sourceRoots: [],
    ...extra,
  })
}

async function started(app: TestApp, extra: Partial<CoordinatorOptions> = {}) {
  const handle = startCoordinator(dev(app, extra))
  await handle.ready
  return { handle, client: connect(app.portFile, app.identity) }
}

afterEach(() => {
  resetCoordinator()
  removeTestApps()
})

describe('removing and renaming sources', () => {
  it('purges a deleted file, global CSS included, without renumbering the rest', async () => {
    const app = createTestApp()
    const a = app.write('src/a.tsx', globalBox('red', 7))
    app.write('src/b.tsx', box('blue'))
    const { client } = await started(app)
    await client.post('/extract', app.post('src/a.tsx'))
    const b = JSON.parse(
      (await client.post('/extract', app.post('src/b.tsx'))).body,
    )
    const before = revision(app)
    expect((await client.get('/css')).body).toContain('margin:28px')

    rmSync(a)
    const base = (await client.get('/css')).body
    const afterB = JSON.parse(
      (await client.post('/extract', app.post('src/b.tsx'))).body,
    )
    const c = JSON.parse(
      (await client.post('/extract', app.post('src/c.tsx', box('green')))).body,
    )
    await flushCoordinatorWrites()

    expect(base).not.toContain('margin:28px')
    expect((await client.get('/css?fileNum=0')).body).not.toContain(
      'background:red',
    )
    expect((await client.get('/css?fileNum=1')).body).toContain(
      'background:blue',
    )
    expect(afterB.code).toBe(b.code)
    expect(c.cssFile).toEndWith('devup-ui-2.css')
    expect(revision(app)).toBeGreaterThan(before)
    const snapshot = readCoordinatorState(stateOf(app), '')
    expect(snapshot?.inputs.map((input) => input.filename)).toEqual([
      'src/b.tsx',
      'src/c.tsx',
    ])
    expect(Object.keys(snapshot?.fileMap ?? {}).sort()).toEqual([
      'src/a.tsx',
      'src/b.tsx',
      'src/c.tsx',
    ])
  })

  it('purges the old name of a renamed file and serves the new one', async () => {
    const app = createTestApp()
    const old = app.write('src/old.tsx', box('red'))
    const { client } = await started(app)
    await client.post('/extract', app.post('src/old.tsx'))

    renameSync(old, join(app.root, 'src', 'renamed.tsx'))
    const renamed = JSON.parse(
      (await client.post('/extract', app.post('src/renamed.tsx', box('blue'))))
        .body,
    )

    expect(renamed.cssFile).toEndWith('devup-ui-1.css')
    expect((await client.get('/css?fileNum=0')).body).not.toContain(
      'background:red',
    )
    expect((await client.get('/css?fileNum=1')).body).toContain(
      'background:blue',
    )
  })

  it('makes its replacement engine for the project itself unless told how', async () => {
    const app = createTestApp()
    const a = app.write('src/a.tsx', box('red'))
    app.write('src/b.tsx', box('blue'))
    const { client } = await started(app, { createEngine: undefined })
    await client.post('/extract', app.post('src/a.tsx'))
    await client.post('/extract', app.post('src/b.tsx'))

    rmSync(a)

    expect((await client.get('/css?fileNum=1')).body).toContain(
      'background:blue',
    )
    expect((await client.get('/css?fileNum=0')).body).not.toContain(
      'background:red',
    )
  })

  it('keeps sources that never had a file on disk', async () => {
    const app = createTestApp()
    const a = app.write('src/a.tsx', box('red'))
    const { client } = await started(app)
    await client.post('/extract', app.post('src/a.tsx'))
    await client.post('/extract', app.post('src/virtual.tsx', box('pink')))

    rmSync(a)

    expect((await client.get('/css?fileNum=1')).body).toContain(
      'background:pink',
    )
    expect((await client.get('/css?fileNum=0')).body).not.toContain(
      'background:red',
    )
  })

  it('rebuilds from the file watcher alone and leaves the revision alone when nothing changed', async () => {
    const app = createTestApp()
    const a = app.write('src/a.tsx', globalBox('red', 7))
    const { client } = await started(app, {
      sourceRoots: [join(app.root, 'src')],
    })
    await client.post('/extract', app.post('src/a.tsx'))
    const before = revision(app)
    await client.post('/extract', app.post('src/a.tsx'))
    await flushCoordinatorWrites()
    expect(revision(app)).toBe(before)

    rmSync(a)
    await eventually(() => (revision(app) > before ? true : undefined))

    expect((await client.get('/css')).body).not.toContain('margin:28px')
  })

  it('reports a rebuild the watcher could not do instead of swallowing it', async () => {
    const app = createTestApp()
    const a = app.write('src/a.tsx', box('red'))
    const errors = spyOn(console, 'error').mockImplementation(() => undefined)
    try {
      const { client } = await started(app, {
        sourceRoots: [join(app.root, 'src')],
        createEngine: () => {
          throw new Error('no engine today')
        },
      })
      await client.post('/extract', app.post('src/a.tsx'))

      rmSync(a)

      const message = await eventually(() =>
        errors.mock.calls
          .map((call) => String(call[1]))
          .find((text) => text.includes('no engine today')),
      )
      expect(message).toContain('no engine today')
      expect(errors.mock.calls[0]?.[0]).toBe('[devup-ui]')
    } finally {
      errors.mockRestore()
    }
  })

  it('keeps serving the old state when the engine cannot be rebuilt, and recovers', async () => {
    const app = createTestApp()
    const a = app.write('src/a.tsx', box('red'))
    app.write('src/b.tsx', box('blue'))
    const wasm = app.engine()
    let next = wasm
    const { client } = await started(app, { wasm, createEngine: () => next })
    await client.post('/extract', app.post('src/a.tsx'))
    await client.post('/extract', app.post('src/b.tsx'))

    rmSync(a)
    const failed = await client.get('/css?fileNum=1')
    next = app.engine()
    const recovered = await client.get('/css?fileNum=1')

    expect(failed.status).toBe(500)
    expect(JSON.parse(failed.body).error).toContain(
      'createEngine returned the engine in service',
    )
    expect(recovered.status).toBe(200)
    expect(recovered.body).toContain('background:blue')
  })

  it('fails the request that replays a broken input, naming the file, and recovers when it is re-sent', async () => {
    const app = createTestApp()
    const gone = app.write('src/gone.tsx', box('red'))
    app.write('src/dep.tsx', box('blue'))
    const { client } = await started(app, {
      createEngine: () => {
        const engine = app.engine()
        return {
          ...engine,
          codeExtract: (filename, code, ...rest) => {
            if (code.includes('blue')) throw new Error('dependency vanished')
            return engine.codeExtract(filename, code, ...rest)
          },
        }
      },
    })
    await client.post('/extract', app.post('src/gone.tsx'))
    await client.post('/extract', app.post('src/dep.tsx'))

    rmSync(gone)
    const failed = await client.get('/css')
    const resent = await client.post(
      '/extract',
      app.post('src/dep.tsx', box('teal')),
    )

    expect(failed.status).toBe(500)
    expect(JSON.parse(failed.body).error).toMatch(
      /^src\/dep\.tsx:1:1: devup-ui coordinator cannot rebuild the CSS state after a source or theme change: dependency vanished\. Fix: /,
    )
    expect(resent.status).toBe(200)
    expect((await client.get('/css?fileNum=1')).body).toContain(
      'background:teal',
    )
  })

  it('does not put a rebuilt engine in service until its state is on disk', async () => {
    const app = createTestApp()
    const a = app.write('src/a.tsx', box('red'))
    app.write('src/b.tsx', box('blue'))
    let engines = 0
    const { client } = await started(app, {
      createEngine: () => {
        engines += 1
        return app.engine()
      },
    })
    await client.post('/extract', app.post('src/a.tsx'))
    await client.post('/extract', app.post('src/b.tsx'))
    await flushCoordinatorWrites()
    rmSync(stateOf(app))
    mkdirSync(stateOf(app))

    rmSync(a)
    const blocked = await client.get('/css?fileNum=1')
    const stillBlocked = await client.get('/css?fileNum=1')
    rmSync(stateOf(app), { recursive: true })
    const recovered = await client.get('/css?fileNum=0')

    expect(blocked.status).toBe(500)
    expect(stillBlocked.status).toBe(500)
    expect(recovered.status).toBe(200)
    expect(recovered.body).not.toContain('background:red')
    expect(engines).toBe(3)
  })
})

describe('resuming from the checkpoint', () => {
  it('replays only the files that still exist and keeps every number handed out', async () => {
    const app = createTestApp()
    const a = app.write('src/a.tsx', globalBox('red', 7))
    app.write('src/b.tsx', box('blue'))
    const first = await started(app)
    await first.client.post('/extract', app.post('src/a.tsx'))
    await first.client.post('/extract', app.post('src/b.tsx'))
    await first.handle.drain()
    rmSync(a)

    const second = await started(app)
    const css = await second.client.get('/css')
    const fresh = JSON.parse(
      (
        await second.client.post(
          '/extract',
          app.post('src/c.tsx', box('green')),
        )
      ).body,
    )

    expect(css.body).not.toContain('margin:28px')
    expect((await second.client.get('/css?fileNum=1')).body).toContain(
      'background:blue',
    )
    expect(fresh.cssFile).toEndWith('devup-ui-2.css')
  })

  it('leaves out a file that changed while it was down; its loader sends it again', async () => {
    const app = createTestApp()
    const a = app.write('src/a.tsx', box('red'))
    app.write('src/b.tsx', box('blue'))
    const first = await started(app)
    await first.client.post('/extract', app.post('src/a.tsx'))
    await first.client.post('/extract', app.post('src/b.tsx'))
    await first.handle.drain()
    writeFileSync(a, box('orange'))

    const second = await started(app)

    expect((await second.client.get('/css?fileNum=0')).body).not.toContain(
      'background',
    )
    expect((await second.client.get('/css?fileNum=1')).body).toContain(
      'background:blue',
    )
    const resent = JSON.parse(
      (await second.client.post('/extract', app.post('src/a.tsx'))).body,
    )
    expect(resent.cssFile).toEndWith('devup-ui-0.css')
    expect((await second.client.get('/css?fileNum=0')).body).toContain(
      'background:orange',
    )
  })

  it('starts cold when the checkpoint belongs to other options', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const first = await started(app, { optionsKey: 'one' })
    await first.client.post('/extract', app.post('src/a.tsx'))
    await first.handle.drain()

    const second = await started(app, { optionsKey: 'two' })
    const fresh = JSON.parse(
      (await second.client.post('/extract', app.post('src/b.tsx', box('blue'))))
        .body,
    )

    expect((await second.client.get('/css?fileNum=0')).body).not.toContain(
      'background:red',
    )
    expect(fresh.cssFile).toEndWith('devup-ui-0.css')
  })

  it('refuses a corrupt checkpoint at startup with a located error', () => {
    const app = createTestApp()
    writeFileSync(stateOf(app), '{"version":1}')

    expect(() => startCoordinator(dev(app))).toThrow(
      /state\.json:1:1: devup-ui coordinator checkpoint cannot use `snapshot` at build time: is corrupt: expected optionsKey string\. Fix: delete this file/,
    )
    expect(existsSync(app.portFile)).toBe(false)
  })

  it('refuses an unreadable theme chain at startup', () => {
    const app = createTestApp()
    app.write('devup.json', '{nope')

    expect(() =>
      startCoordinator(dev(app, { devupFile: 'devup.json' })),
    ).toThrow(/devup\.json:1:1: devup config cannot use `JSON` at build time/)
  })
})

describe('theme changes', () => {
  const themed = (color: string) => ({
    theme: { colors: { default: { primary: color } } },
  })

  it('applies the current theme when devup.json or what it extends changes', async () => {
    const app = createTestApp()
    app.write('base.json', JSON.stringify(themed('#f00')))
    app.write('devup.json', JSON.stringify({ extends: ['./base.json'] }))
    app.write(
      'src/a.tsx',
      'import { Box } from \'@devup-ui/react\'\nexport const A = () => <Box color="$primary" />\n',
    )
    const wasm = app.engine()
    wasm.registerTheme(themed('#f00').theme)
    const configured: DevupWasm[] = []
    const { client } = await started(app, {
      wasm,
      devupFile: 'devup.json',
      configureWasm: (engine) => configured.push(engine),
    })

    const reply = JSON.parse(
      (await client.post('/extract', app.post('src/a.tsx'))).body,
    )
    const first = (await client.get('/css')).body
    writeFileSync(join(app.root, 'base.json'), JSON.stringify(themed('#00f')))
    const second = (await client.get('/css')).body
    const again = JSON.parse(
      (await client.post('/extract', app.post('src/a.tsx'))).body,
    )

    expect(reply.dependencies).toEqual([
      join(app.root, 'devup.json'),
      join(app.root, 'base.json'),
    ])
    expect(first).toContain('--primary:#F00')
    expect(second).toContain('--primary:#00F')
    expect(second).not.toContain('#F00')
    expect(again.code).toBe(reply.code)
    expect(configured).toHaveLength(1)
    expect(configured[0]).not.toBe(wasm)
  })

  it('refuses to serve while the theme chain is unreadable, then recovers', async () => {
    const app = createTestApp()
    const config = app.write('devup.json', JSON.stringify(themed('#f00')))
    const { client } = await started(app, { devupFile: 'devup.json' })

    writeFileSync(config, '{nope')
    const broken = await client.get('/css')
    writeFileSync(config, JSON.stringify(themed('#0f0')))
    const fixed = await client.get('/css')

    expect(broken.status).toBe(500)
    expect(JSON.parse(broken.body).error).toContain(
      'devup.json:1:1: devup config cannot use `JSON` at build time',
    )
    expect(fixed.status).toBe(200)
  })
})

describe('identical transforms', () => {
  async function counted(
    app: TestApp,
    extra: Partial<CoordinatorOptions> = {},
  ) {
    const { engine, extractions } = instrument(app.engine())
    const { client } = await started(app, {
      wasm: engine,
      createEngine: () => instrument(app.engine(), extractions).engine,
      ...extra,
    })
    return { client, extractions }
  }

  it("runs the engine once for the same source from Next's server and client graphs", async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const { client, extractions } = await counted(app)

    const replies = await Promise.all([
      client.post('/extract', app.post('src/a.tsx')),
      client.post('/extract', app.post('src/a.tsx')),
    ])
    await client.post('/extract', app.post('src/a.tsx'))

    expect(replies[0]?.body).toBe(replies[1]?.body)
    expect(extractions).toEqual(['map:src/a.tsx'])
  })

  it('runs the engine again once the source differs from what it holds, even if it was seen before', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const { client, extractions } = await counted(app)

    await client.post('/extract', app.post('src/a.tsx'))
    await client.post('/extract', app.post('src/a.tsx', box('blue')))
    await client.post('/extract', app.post('src/a.tsx'))

    expect(extractions).toHaveLength(3)
    expect((await client.get('/css?fileNum=0')).body).toContain(
      'background:red',
    )
  })

  it('validates what the source imported before reusing its output', async () => {
    const app = createTestApp()
    const tokens = app.write('src/tokens.ts', "export const color = 'red'\n")
    app.write(
      'src/dep.tsx',
      "import { Box } from '@devup-ui/react'\nimport { color } from './tokens'\nexport const D = () => <Box bg={color} />\n",
    )
    const { client, extractions } = await counted(app)
    const first = JSON.parse(
      (await client.post('/extract', app.post('src/dep.tsx'))).body,
    )

    writeFileSync(tokens, "export const color = 'red'\n")
    await client.post('/extract', app.post('src/dep.tsx'))
    const unchanged = extractions.length
    writeFileSync(tokens, "export const color = 'blue'\n")
    await client.post('/extract', app.post('src/dep.tsx'))

    expect(first.dependencies).toEqual(['src/tokens.ts'])
    expect(unchanged).toBe(1)
    expect(extractions).toHaveLength(2)
    expect((await client.get('/css?fileNum=0')).body).toContain(
      'background:blue',
    )
  })

  it('bounds the cache and re-extracts what it dropped', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    app.write('src/b.tsx', box('blue'))
    const { client, extractions } = await counted(app, { cacheMaxEntries: 1 })

    await client.post('/extract', app.post('src/a.tsx'))
    await client.post('/extract', app.post('src/b.tsx'))
    await client.post('/extract', app.post('src/b.tsx'))
    await client.post('/extract', app.post('src/a.tsx'))

    expect(extractions).toEqual([
      'map:src/a.tsx',
      'map:src/b.tsx',
      'map:src/a.tsx',
    ])
  })

  it('starts over after a rebuild because the new engine holds nothing yet', async () => {
    const app = createTestApp()
    const gone = app.write('src/gone.tsx', box('red'))
    app.write('src/a.tsx', box('blue'))
    const { client, extractions } = await counted(app)
    await client.post('/extract', app.post('src/gone.tsx'))
    await client.post('/extract', app.post('src/a.tsx'))

    rmSync(gone)
    await client.get('/css')
    await client.post('/extract', app.post('src/a.tsx'))

    expect(
      extractions.filter((entry) => entry === 'map:src/a.tsx'),
    ).toHaveLength(3)
  })
})
