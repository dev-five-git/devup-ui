import { mkdirSync, rmSync } from 'node:fs'
import * as fsp from 'node:fs/promises'
import { join } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import {
  type CoordinatorOptions,
  resetCoordinator,
  startCoordinator,
  takeExtractOutput,
} from '../coordinator'
import { readCoordinatorState } from '../state'
import {
  connect,
  createTestApp,
  instrument,
  removeTestApps,
  type TestApp,
} from './coordinator-app'

const box = (bg: string) =>
  `import { Box } from '@devup-ui/react'\nexport const C = () => <Box bg="${bg}" p={4} />\n`
const bad =
  "import { css } from '@devup-ui/react'\nexport const x = css(foo())\n"

const WAIT = '/css?fileNum=0&importMainCss=true&waitForIdle=true'
const BASE = '/css?waitForIdle=true'

async function started(app: TestApp, extra: Partial<CoordinatorOptions> = {}) {
  const handle = startCoordinator(app.options(extra))
  await handle.ready
  return { handle, client: connect(app.portFile, app.identity) }
}

afterEach(() => {
  resetCoordinator()
  removeTestApps()
})

describe('production completeness', () => {
  it('serves a prewarmed stylesheet, even an empty one, without any loader request', async () => {
    const app = createTestApp()
    const { client } = await started(app, { prewarmedFiles: [] })

    const reply = await client.get(BASE)

    expect(reply.status).toBe(200)
    expect(reply.headers['x-devup-css-policy']).toBe('production-complete')
    expect(reply.body).toContain('devup-ui')
  })

  it('refuses to guess completeness when there is no plan', async () => {
    const app = createTestApp()
    const { client } = await started(app)

    const reply = await client.get(BASE)

    expect(reply.status).toBe(500)
    expect(JSON.parse(reply.body).error).toMatch(
      /^devup-ui\.css:1:1: devup-ui coordinator cannot serve a complete production stylesheet: neither prewarmedFiles nor expectedBaseFiles/,
    )
  })

  it('refuses a production stylesheet request that does not wait', async () => {
    const app = createTestApp()
    const { client } = await started(app, { prewarmedFiles: [] })

    const reply = await client.get('/css')

    expect(reply.status).toBe(400)
    expect(JSON.parse(reply.body).error).toContain(
      'request /css with waitForIdle=true',
    )
  })

  it('names the missing files instead of serving partial CSS after the bounded wait', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const { client } = await started(app, {
      expectedBaseFiles: ['src/a.tsx', 'src/b.tsx'],
      maxWaitMs: 100,
    })
    await client.post('/extract', app.post('src/a.tsx'))

    const reply = await client.get(BASE)

    expect(reply.status).toBe(500)
    const body = JSON.parse(reply.body)
    expect(body.missing).toEqual(['src/b.tsx'])
    expect(body.error).toStartWith(
      'src/b.tsx:1:1: devup-ui coordinator cannot use `the base stylesheet` at build time:',
    )
    expect(body.error).toContain('never extracted: src/b.tsx')
  })

  it('completes a waiting request when the last planned file arrives', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    app.write('src/b.tsx', box('blue'))
    const { client } = await started(app, {
      expectedBaseFiles: ['src/a.tsx', 'src/b.tsx'],
    })
    await client.post('/extract', app.post('src/a.tsx'))

    const waiting = client.get(BASE)
    await delay(100)
    await client.post('/extract', app.post('src/b.tsx'))

    expect((await waiting).status).toBe(200)
    expect((await client.get(WAIT)).body).toContain('background:red')
  })

  it('fails at once for a planned file whose extraction failed, until it extracts', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', bad)
    const { client } = await started(app, { expectedBaseFiles: ['src/a.tsx'] })

    const waiting = client.get(BASE)
    await delay(50)
    const extracted = await client.post('/extract', app.post('src/a.tsx'))
    const failed = await waiting
    const again = await client.get(BASE)
    const fixed = await client.post(
      '/extract',
      app.post('src/a.tsx', box('red')),
    )

    expect(extracted.status).toBe(500)
    for (const reply of [failed, again]) {
      expect(reply.status).toBe(500)
      expect(JSON.parse(reply.body).failed).toEqual(['src/a.tsx'])
      expect(JSON.parse(reply.body).error).toContain(
        'extraction failed for src/a.tsx',
      )
    }
    expect(fixed.status).toBe(200)
    expect((await client.get(BASE)).status).toBe(200)
  })

  it('waits for every member of a collapsed bucket, and reads `@global` and unknown numbers as base', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    app.write('src/b.tsx', box('blue'))
    const { client } = await started(app, {
      canonicalMap: {
        'src/a.tsx': 'src/a.tsx',
        'src/b.tsx': 'src/a.tsx',
        'src/g.tsx': '@global',
      },
      prewarmedFiles: [],
      maxWaitMs: 2000,
    })
    await client.post('/extract', app.post('src/a.tsx'))

    const waiting = client.get(WAIT)
    await delay(100)
    await client.post('/extract', app.post('src/b.tsx'))

    expect((await waiting).status).toBe(200)
    expect((await client.get('/css?fileNum=99&waitForIdle=true')).status).toBe(
      200,
    )
  })

  it('names the members of a bucket that never completes', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const { client } = await started(app, {
      canonicalMap: { 'src/a.tsx': 'src/a.tsx', 'src/b.tsx': 'src/a.tsx' },
      maxWaitMs: 100,
    })
    await client.post('/extract', app.post('src/a.tsx'))

    const reply = await client.get(WAIT)

    expect(reply.status).toBe(500)
    expect(JSON.parse(reply.body).missing).toEqual(['src/b.tsx'])
  })

  it('serves a bucket whose only member was prewarmed', async () => {
    const app = createTestApp()
    const source = box('red')
    const engine = app.engine()
    const output = takeExtractOutput(
      engine.codeExtract(
        'src/a.tsx',
        source,
        '@devup-ui/react',
        './df',
        false,
        false,
        true,
        {},
      ),
    )
    const { client } = await started(app, {
      wasm: engine,
      prewarmedFiles: ['src/a.tsx'],
      prewarmedOutputs: new Map([['src/a.tsx', { ...output, source }]]),
    })

    const reply = await client.get(WAIT)

    expect(reply.status).toBe(200)
    expect(reply.body).toContain('background:red')
  })
})

describe('after the stylesheet was served', () => {
  async function sealed() {
    const app = createTestApp()
    const source = box('red')
    app.write('src/a.tsx', source)
    const prewarm = app.engine()
    const prewarmed = takeExtractOutput(
      prewarm.codeExtract(
        'src/a.tsx',
        source,
        '@devup-ui/react',
        './df',
        false,
        false,
        true,
        {},
      ),
    )
    const { engine, extractions } = instrument(prewarm)
    const stateFile = join(app.root, 'df', 'state.json')
    const { client } = await started(app, {
      wasm: engine,
      createEngine: () => instrument(app.engine(), extractions).engine,
      stateFile,
      expectedBaseFiles: ['src/a.tsx'],
      prewarmedFiles: ['src/a.tsx'],
      prewarmedOutputs: new Map([['src/a.tsx', { ...prewarmed, source }]]),
    })
    return { app, client, extractions, stateFile }
  }

  it('takes unplanned files until then', async () => {
    const { app, client } = await sealed()

    const early = await client.post(
      '/extract',
      app.post('src/c.tsx', box('green')),
    )

    expect(early.status).toBe(200)
  })

  it('returns exact prewarm outputs without running the engine', async () => {
    const { app, client, extractions } = await sealed()
    await client.get(WAIT)

    const exact = await client.post('/extract', app.post('src/a.tsx'))

    expect(exact.status).toBe(200)
    expect(extractions).toEqual([])
  })

  it('allows a changed source that produces the same CSS', async () => {
    const { app, client, extractions } = await sealed()
    await client.get(WAIT)

    const same = await client.post(
      '/extract',
      app.post('src/a.tsx', `${box('red')}// touched\n`),
    )

    expect(same.status).toBe(200)
    expect(extractions).toEqual(['map:src/a.tsx'])
  })

  it('rejects unplanned or changed styles, naming the file and the fix', async () => {
    const { app, client } = await sealed()
    await client.get(WAIT)

    const unplanned = await client.post(
      '/extract',
      app.post('src/c.tsx', box('green')),
    )
    const changed = await client.post(
      '/extract',
      app.post('src/a.tsx', box('blue')),
    )

    for (const reply of [unplanned, changed]) {
      expect(reply.status).toBe(500)
    }
    expect(JSON.parse(unplanned.body).error).toMatch(
      /^src\/c\.tsx:1:1: devup-ui coordinator cannot change styles after the production stylesheet was served: .+ Fix: include it in expectedBaseFiles/,
    )
  })
})

describe('persistence acknowledgement', () => {
  it('does not acknowledge an extraction whose state could not be written, and retries on repeat', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const stateFile = join(app.root, 'df', 'state.json')
    const { client } = await started(app, {
      stateFile,
      expectedBaseFiles: ['src/a.tsx'],
    })
    rmSync(stateFile)
    mkdirSync(stateFile)

    const failed = await client.post('/extract', app.post('src/a.tsx'))
    const incomplete = await client.get(BASE)
    rmSync(stateFile, { recursive: true })
    const repeated = await client.post('/extract', app.post('src/a.tsx'))

    expect(failed.status).toBe(500)
    expect(incomplete.status).toBe(500)
    expect(JSON.parse(incomplete.body).failed).toEqual(['src/a.tsx'])
    expect(repeated.status).toBe(200)
    expect(readCoordinatorState(stateFile, '')?.inputs).toHaveLength(1)
    expect((await client.get(BASE)).status).toBe(200)
  })

  it('holds back every identical request until the shared write has landed', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const stateFile = join(app.root, 'df', 'state.json')
    const { client } = await started(app, { stateFile })
    const original = fsp.rename
    const spy = spyOn(fsp, 'rename').mockImplementation(async (...args) => {
      await delay(250)
      return original(...args)
    })

    try {
      const startedAt = performance.now()
      const replies = await Promise.all([
        client.post('/extract', app.post('src/a.tsx')),
        client.post('/extract', app.post('src/a.tsx')),
      ])

      expect(replies.map((reply) => reply.status)).toEqual([200, 200])
      expect(performance.now() - startedAt).toBeGreaterThanOrEqual(200)
    } finally {
      spy.mockRestore()
    }
    expect(readCoordinatorState(stateFile, '')?.inputs).toHaveLength(1)
  })
})
