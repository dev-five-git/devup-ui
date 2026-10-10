import { existsSync, readFileSync } from 'node:fs'
import * as fsp from 'node:fs/promises'
import { Server } from 'node:http'
import { join } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

import { afterEach, describe, expect, it, mock, spyOn } from 'bun:test'

import {
  flushCoordinatorWrites,
  resetCoordinator,
  startCoordinator,
  takeExtractOutput,
} from '../coordinator'
import { parsePortFile } from '../coordinator-port'
import { readCoordinatorState } from '../state'
import {
  connect,
  createTestApp,
  eventually,
  failure,
  http,
  instrument,
  ownership,
  removeTestApps,
} from './coordinator-app'

const box = (bg: string) =>
  `import { Box } from '@devup-ui/react'\nexport const C = () => <Box bg="${bg}" p={4} />\n`

it.each([
  undefined,
  'code',
  'css',
  'cssFile',
  'map',
  'updatedBaseStyle',
  'dependencies',
])('preserves modern Next snapshot cleanup when getter %s fails', (fault) => {
  const events: string[] = []
  const error = new Error('getter fault')
  let live = true
  const free = mock(() => {
    expect(live).toBe(true)
    live = false
    events.push('free')
  })
  const output = {
    code: 'copied',
    css: 'sheet',
    cssFile: 'sheet.css',
    map: 'map',
    updatedBaseStyle: true,
    dependencies: ['dep.ts'],
    free,
    [Symbol.dispose]: free,
  }
  const expected = {
    code: 'copied',
    css: 'sheet',
    cssFile: 'sheet.css',
    map: 'map',
    updatedBaseStyle: true,
    dependencies: ['dep.ts'],
  }
  const fields = [
    'code',
    'css',
    'cssFile',
    'map',
    'updatedBaseStyle',
    'dependencies',
  ] as const
  for (const field of fields) {
    const value = output[field]
    Object.defineProperty(output, field, {
      get() {
        expect(live).toBe(true)
        events.push(field)
        if (fault === field) throw error
        return value
      },
    })
  }
  if (fault) expect(() => takeExtractOutput(output)).toThrow(error)
  else expect(takeExtractOutput(output)).toEqual(expected)
  expect(events).toEqual([
    ...fields.slice(0, fault ? fields.indexOf(fault) + 1 : fields.length),
    'free',
  ])
  expect(free).toHaveBeenCalledTimes(1)
})

afterEach(() => {
  resetCoordinator()
  removeTestApps()
})

describe('endpoint', () => {
  it('publishes its identity and answers health with the same descriptor', async () => {
    const app = createTestApp()
    const handle = startCoordinator(app.options())
    await handle.ready

    const client = connect(app.portFile, app.identity)
    const health = await client.get('/health')

    expect(health.status).toBe(200)
    expect(parsePortFile(health.body)).toEqual(client.info)
    expect(client.info).toMatchObject({ ...app.identity, pid: process.pid })
    handle.close()
    expect(existsSync(app.portFile)).toBe(false)
  })

  it('extracts through its own engine and rewrites per-file CSS imports', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const handle = startCoordinator(app.options())
    await handle.ready
    const client = connect(app.portFile, app.identity)

    const reply = await client.post('/extract', app.post('src/a.tsx'))

    expect(reply.status).toBe(200)
    const data = JSON.parse(reply.body)
    expect(data.code).toContain('devup-ui.css?fileNum=0')
    expect(data.cssFile).toEndWith('devup-ui-0.css')
    expect(JSON.parse(data.map).version).toBe(3)
    expect(data.dependencies).toEqual([])
  })

  it('leaves single-CSS imports alone and skips source maps when asked', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const { engine, extractions } = instrument(app.engine())
    const handle = startCoordinator(
      app.options({ wasm: engine, singleCss: true, sourceMap: false }),
    )
    await handle.ready
    const client = connect(app.portFile, app.identity)

    const data = JSON.parse(
      (await client.post('/extract', app.post('src/a.tsx'))).body,
    )

    expect(data.code).not.toContain('fileNum')
    expect(extractions).toEqual(['nomap:src/a.tsx'])
  })

  it('serves the current CSS in development and labels the policy', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const handle = startCoordinator(app.options({ watch: true }))
    await handle.ready
    const client = connect(app.portFile, app.identity)
    await client.post('/extract', app.post('src/a.tsx'))

    const bucket = await client.get('/css?fileNum=0&importMainCss=true')

    expect(bucket.status).toBe(200)
    expect(bucket.headers['content-type']).toBe('text/css')
    expect(bucket.headers['x-devup-css-policy']).toBe('dev-current')
    expect(bucket.body).toContain('background:red')
    expect(bucket.body).toContain('@import')
    expect((await client.get('/css')).body).not.toContain('background:red')
  })

  it('answers unknown routes with 404', async () => {
    const app = createTestApp()
    const handle = startCoordinator(app.options())
    await handle.ready

    const reply = await connect(app.portFile, app.identity).get('/nope')

    expect(reply).toMatchObject({ status: 404, body: 'Not Found' })
  })

  it('rejects malformed requests with located errors', async () => {
    const app = createTestApp()
    const handle = startCoordinator(app.options())
    await handle.ready
    const client = connect(app.portFile, app.identity)

    const notJson = await client.post('/extract', '{nope')
    const wrongShape = await client.post('/extract', '{"filename":1}')
    const badFileNum = await client.get('/css?fileNum=x1')

    expect(notJson.status).toBe(400)
    expect(JSON.parse(notJson.body).error).toStartWith('/extract:1:1:')
    expect(wrongShape.status).toBe(400)
    expect(JSON.parse(wrongShape.body).error).toContain(
      'filename, code and resourcePath must all be strings',
    )
    expect(badFileNum.status).toBe(400)
    expect(JSON.parse(badFileNum.body).error).toStartWith('/css:1:1:')
  })

  it('reports build errors with their position, adding one when the engine has none', async () => {
    const app = createTestApp()
    app.write(
      'src/dyn.tsx',
      "import { css } from '@devup-ui/react'\nexport const x = css(foo())\n",
    )
    app.write(
      'src/broken.tsx',
      'import { Box } from \'@devup-ui/react\'\nexport const A = () => <Box bg="red"',
    )
    const handle = startCoordinator(app.options())
    await handle.ready
    const client = connect(app.portFile, app.identity)

    const positioned = await client.post('/extract', app.post('src/dyn.tsx'))
    const unpositioned = await client.post(
      '/extract',
      app.post('src/broken.tsx'),
    )

    expect(positioned.status).toBe(500)
    expect(JSON.parse(positioned.body).error).toStartWith(
      'src/dyn.tsx:2:18: Cannot compose `foo()` at build time',
    )
    expect(unpositioned.status).toBe(500)
    expect(JSON.parse(unpositioned.body).error).toMatch(
      /^src\/broken\.tsx:1:1: devup-ui coordinator cannot extract styles: .+ Fix: /,
    )
  })
})

describe('ownership', () => {
  it('requires matching project and token headers once an identity is given', async () => {
    const app = createTestApp()
    const handle = startCoordinator(app.options())
    await handle.ready
    const { info } = connect(app.portFile, app.identity)
    const right = ownership(app.identity)

    const none = await http(info.port, 'GET', '/health')
    const onlyProject = await http(info.port, 'GET', '/health', {
      headers: { 'x-devup-project': right['x-devup-project'] },
    })
    const wrongToken = await http(info.port, 'GET', '/health', {
      headers: {
        ...right,
        'x-devup-token': '00000000-0000-4000-8000-000000000000',
      },
    })
    const wrongProject = await http(info.port, 'POST', '/extract', {
      headers: { ...right, 'x-devup-project': join(app.root, 'other') },
      body: app.post('src/a.tsx', box('red')),
    })
    const accepted = await http(info.port, 'GET', '/health', { headers: right })

    for (const refused of [none, onlyProject, wrongToken, wrongProject]) {
      expect(refused.status).toBe(403)
      expect(JSON.parse(refused.body).error).toContain(
        ':1:1: devup-ui coordinator',
      )
    }
    expect(accepted.status).toBe(200)
  })

  it('lets header-less direct callers in only when no identity was given', async () => {
    const app = createTestApp()
    const handle = startCoordinator(app.options({ identity: undefined }))
    await handle.ready
    const port = parsePortFile(readFileSync(app.portFile, 'utf-8')).port

    const plain = await http(port, 'GET', '/health')
    const foreign = await http(port, 'GET', '/health', {
      headers: ownership({ project: app.root, token: 'someone-else' }),
    })

    expect(plain.status).toBe(200)
    expect(foreign.status).toBe(403)
  })
})

describe('instances', () => {
  it('lets two apps in one process run side by side with their own engines', async () => {
    const first = createTestApp()
    const second = createTestApp()
    first.write('src/a.tsx', box('red'))
    second.write('src/a.tsx', box('blue'))
    const firstHandle = startCoordinator(first.options({ watch: true }))
    const secondHandle = startCoordinator(second.options({ watch: true }))
    await Promise.all([firstHandle.ready, secondHandle.ready])
    const one = connect(first.portFile, first.identity)
    const two = connect(second.portFile, second.identity)
    await one.post('/extract', first.post('src/a.tsx'))
    await two.post('/extract', second.post('src/a.tsx'))

    secondHandle.close()

    expect(one.info.port).not.toBe(two.info.port)
    expect((await one.get('/css?fileNum=0')).body).toContain('background:red')
    expect((await one.get('/health')).status).toBe(200)
    expect(existsSync(second.portFile)).toBe(false)
    expect(await failure(two.get('/health'))).toBeInstanceOf(Error)
    expect(existsSync(first.portFile)).toBe(true)
  })

  it('shares one instance between handles of the same identity until the last is released', async () => {
    const app = createTestApp()
    const first = startCoordinator(app.options())
    const second = startCoordinator(app.options())
    await first.ready
    const client = connect(app.portFile, app.identity)

    first.close()
    first.close()
    expect((await client.get('/health')).status).toBe(200)
    await second.drain()

    expect(first.ready).toBe(second.ready)
    expect(await failure(client.get('/health'))).toBeInstanceOf(Error)
    expect(existsSync(app.portFile)).toBe(false)
  })

  it('refuses to share an endpoint with an unrelated app instead of closing it', async () => {
    const app = createTestApp()
    const owner = startCoordinator(app.options())
    await owner.ready
    const client = connect(app.portFile, app.identity)

    const stranger = {
      project: app.identity.project,
      token: '11111111-1111-4111-8111-111111111111',
    }

    expect(() => startCoordinator(app.options({ identity: stranger }))).toThrow(
      /coordinator\.port:1:1: devup-ui coordinator cannot start: another coordinator/,
    )
    expect(() =>
      startCoordinator(app.options({ identity: undefined })),
    ).toThrow('cannot start')
    expect((await client.get('/health')).status).toBe(200)
  })

  it('does not let a closed coordinator delete the port file of its replacement', async () => {
    const app = createTestApp()
    const old = startCoordinator(app.options())
    await old.ready
    old.close()
    const replacement = startCoordinator(
      app.options({
        identity: {
          ...app.identity,
          token: '22222222-2222-4222-8222-222222222222',
        },
      }),
    )
    await replacement.ready

    old.close()

    expect(existsSync(app.portFile)).toBe(true)
    expect(readFileSync(app.portFile, 'utf-8')).toContain('22222222')
  })
})

describe('lifecycle', () => {
  it('does not publish an endpoint when closed before it listens', async () => {
    const app = createTestApp()
    const handle = startCoordinator(app.options())

    handle.close()
    await handle.ready
    await delay(50)

    expect(existsSync(app.portFile)).toBe(false)
  })

  it('does not publish an endpoint when closed while the socket opens', async () => {
    const app = createTestApp()
    const handle = startCoordinator(app.options())
    const listen = Server.prototype.listen
    const spy = spyOn(Server.prototype, 'listen').mockImplementation(function (
      this: Server,
      ...args: unknown[]
    ) {
      const result: Server = Reflect.apply(listen, this, args)
      handle.close()
      return result
    })

    await handle.ready
    const listens = spy.mock.calls.length
    spy.mockRestore()
    await delay(50)

    expect(listens).toBe(1)
    expect(existsSync(app.portFile)).toBe(false)
  })

  it('fails ready, and drain, when the engine cannot be rebuilt at startup', async () => {
    const app = createTestApp()
    const stateFile = join(app.root, 'df', 'state.json')
    const first = startCoordinator(app.options({ watch: true, stateFile }))
    await first.drain()
    const second = startCoordinator(
      app.options({
        watch: true,
        stateFile,
        createEngine: () => {
          throw new Error('engine unavailable')
        },
      }),
    )

    expect(String(await failure(second.ready))).toContain('engine unavailable')
    expect(String(await failure(second.drain()))).toContain(
      'engine unavailable',
    )
    expect(existsSync(app.portFile)).toBe(false)
  })

  it('drain waits for an accepted write, refuses new requests, then closes', async () => {
    const app = createTestApp()
    app.write('src/a.tsx', box('red'))
    const stateFile = join(app.root, 'df', 'state.json')
    const handle = startCoordinator(app.options({ stateFile }))
    await handle.ready
    const client = connect(app.portFile, app.identity)
    const original = fsp.rename
    let renaming = 0
    const spy = spyOn(fsp, 'rename').mockImplementation(async (...args) => {
      renaming += 1
      await delay(300)
      return original(...args)
    })

    try {
      const extract = client.post('/extract', app.post('src/a.tsx'))
      await eventually(() => (renaming > 0 ? true : undefined))
      let drained = false
      const drain = handle.drain().then(() => {
        drained = true
      })
      const refused = await client.get('/health')

      await delay(50)
      expect(drained).toBe(false)
      expect(refused.status).toBe(503)
      expect(JSON.parse(refused.body).error).toContain('shutting down')
      await drain
      expect((await extract).status).toBe(200)
    } finally {
      spy.mockRestore()
    }
    expect(
      readCoordinatorState(stateFile, '')?.inputs.map(
        (input) => input.filename,
      ),
    ).toEqual(['src/a.tsx'])
    expect(existsSync(app.portFile)).toBe(false)
  })

  it('flushCoordinatorWrites waits for every live coordinator', async () => {
    const apps = [createTestApp(), createTestApp()]
    const states = apps.map((app) => join(app.root, 'df', 'state.json'))
    for (const app of apps) app.write('src/a.tsx', box('red'))
    const handles = apps.map((app, i) =>
      startCoordinator(app.options({ stateFile: states[i] })),
    )
    await Promise.all(handles.map((handle) => handle.ready))
    const original = fsp.rename
    const spy = spyOn(fsp, 'rename').mockImplementation(async (...args) => {
      await delay(200)
      return original(...args)
    })

    try {
      const posts = apps.map((app) =>
        connect(app.portFile, app.identity).post(
          '/extract',
          app.post('src/a.tsx'),
        ),
      )
      await delay(50)
      await flushCoordinatorWrites()
      expect(await Promise.all(posts)).toHaveLength(2)
    } finally {
      spy.mockRestore()
    }
    for (const state of states) {
      expect(readCoordinatorState(state, '')?.inputs).toHaveLength(1)
    }
  })

  it('resetCoordinator stops every live coordinator', async () => {
    const apps = [createTestApp(), createTestApp()]
    await Promise.all(apps.map((app) => startCoordinator(app.options()).ready))

    resetCoordinator()

    for (const app of apps) expect(existsSync(app.portFile)).toBe(false)
  })
})
