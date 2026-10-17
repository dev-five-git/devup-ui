import { randomUUID } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs'
import { request, Server } from 'node:http'
import { join } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import {
  CoordinatorShutdownError,
  createCompletionTracker,
} from '../coordinator-completion'
import {
  type CoordinatorInstance,
  createInstance,
} from '../coordinator-instance'
import { readCoordinatorState } from '../state'
import {
  connect,
  createTestApp,
  failure,
  ownership,
  removeTestApps,
} from './coordinator-app'

const instances: CoordinatorInstance[] = []

afterEach(() => {
  for (const instance of instances.splice(0)) instance.close()
  removeTestApps()
})

describe('shutdown cancellation', () => {
  it('rejects pending and future waits and clears their timers when closed', async () => {
    const tracker = createCompletionTracker(60_000)
    const clear = spyOn(globalThis, 'clearTimeout')
    const waiting = failure(tracker.wait('devup-ui.css', ['src/a.tsx']))

    tracker.close()

    expect(await waiting).toBeInstanceOf(CoordinatorShutdownError)
    expect(clear).toHaveBeenCalledTimes(1)
    clear.mockRestore()
    expect(await failure(tracker.wait('devup-ui.css', []))).toBeInstanceOf(
      CoordinatorShutdownError,
    )
    tracker.close()
  })

  it('closes an active CSS socket without waiting for the completion timeout', async () => {
    const app = createTestApp()
    const instance = createInstance(
      app.options({ expectedBaseFiles: ['src/a.tsx'] }),
    )
    instances.push(instance)
    await instance.ready
    const client = connect(app.portFile, app.identity)
    const registered = Promise.withResolvers<void>()
    const emit = Server.prototype.emit
    const observed = spyOn(Server.prototype, 'emit').mockImplementation(
      function (this: Server, event: string | symbol, ...args: unknown[]) {
        const result: boolean = Reflect.apply(emit, this, [event, ...args])
        if (event === 'request') registered.resolve()
        return result
      },
    )
    const waiting = failure(client.get('/css?waitForIdle=true'))
    try {
      await registered.promise

      instance.close()

      expect(await waiting).toBeInstanceOf(Error)
      await instance.flush()
      expect(existsSync(app.portFile)).toBe(false)
    } finally {
      observed.mockRestore()
    }
  })

  it.each(['close', 'drain'] as const)(
    '%s cancels a request still sending its body',
    async (operation) => {
      const app = createTestApp()
      const instance = createInstance(app.options())
      instances.push(instance)
      await instance.ready
      const client = connect(app.portFile, app.identity)
      const received = Promise.withResolvers<void>()
      const emit = Server.prototype.emit
      const observed = spyOn(Server.prototype, 'emit').mockImplementation(
        function (this: Server, event: string | symbol, ...args: unknown[]) {
          const result: boolean = Reflect.apply(emit, this, [event, ...args])
          if (event === 'request') received.resolve()
          return result
        },
      )
      const failed = Promise.withResolvers<Error>()
      const req = request({
        hostname: '127.0.0.1',
        port: client.info.port,
        path: '/extract',
        method: 'POST',
        agent: false,
        headers: { ...ownership(app.identity), 'content-length': '100000' },
      })
      req.on('error', failed.resolve)
      req.write('{')
      await received.promise
      observed.mockRestore()

      const closing = instance[operation]()

      expect(await failed.promise).toBeInstanceOf(Error)
      await closing
      await instance.flush()
      expect(existsSync(app.portFile)).toBe(false)
    },
  )

  it('leaves a replacement endpoint and token intact when the old instance closes', async () => {
    const app = createTestApp()
    const old = createInstance(app.options())
    instances.push(old)
    await old.ready
    const replacementIdentity = { ...app.identity, token: randomUUID() }
    const replacement = createInstance(
      app.options({ identity: replacementIdentity }),
    )
    instances.push(replacement)
    await replacement.ready
    const descriptor = readFileSync(app.portFile, 'utf-8')

    old.close()

    expect(readFileSync(app.portFile, 'utf-8')).toBe(descriptor)
    expect(
      (await connect(app.portFile, replacementIdentity).get('/health')).status,
    ).toBe(200)
    expect(
      (await connect(app.portFile, app.identity).get('/health')).status,
    ).toBe(403)
  })
})

describe('failed durability', () => {
  it('restores durability with a later full capture including the failed input', async () => {
    const app = createTestApp()
    const stateFile = join(app.root, 'df', 'snapshot.json')
    const instance = createInstance(app.options({ stateFile }))
    instances.push(instance)
    await instance.ready
    const client = connect(app.portFile, app.identity)
    rmSync(stateFile)
    mkdirSync(stateFile)
    expect(
      (await client.post('/extract', app.post('a.tsx', 'export const a = 1')))
        .status,
    ).toBe(500)
    expect(await failure(instance.flush())).toBeInstanceOf(Error)
    rmSync(stateFile, { recursive: true })

    const recovered = await client.post(
      '/extract',
      app.post('b.tsx', 'export const b = 2'),
    )

    expect(recovered.status).toBe(200)
    await instance.drain()
    expect(
      readCoordinatorState(stateFile, '')?.inputs.map((input) => input.source),
    ).toEqual(['export const a = 1', 'export const b = 2'])
  })

  it('rejects drain when the accepted snapshot could not be committed', async () => {
    const app = createTestApp()
    const stateFile = join(app.root, 'df', 'snapshot.json')
    const instance = createInstance(app.options({ stateFile }))
    instances.push(instance)
    await instance.ready
    rmSync(stateFile)
    mkdirSync(stateFile)
    const client = connect(app.portFile, app.identity)
    const reply = await client.post(
      '/extract',
      app.post('a.tsx', 'export const a = 1'),
    )
    expect(reply.status).toBe(500)

    const error = await failure(instance.drain())

    expect(String(error)).toContain(`${stateFile}:1:1:`)
    expect(String(error)).toContain('cannot use `snapshot` at build time:')
    expect(existsSync(app.portFile)).toBe(false)
  })

  it('retries a full capture after revision publication fails', async () => {
    const app = createTestApp()
    const stateFile = join(app.root, 'df', 'snapshot.json')
    const revisionFile = join(app.root, 'df', 'revision')
    const instance = createInstance(app.options({ stateFile, revisionFile }))
    instances.push(instance)
    await instance.ready
    rmSync(revisionFile)
    mkdirSync(revisionFile)
    const client = connect(app.portFile, app.identity)
    const source =
      'import { Box } from "@devup-ui/react"; export const A = <Box color="red" />'
    expect(
      (await client.post('/extract', app.post('a.tsx', source))).status,
    ).toBe(500)
    expect(String(await failure(instance.flush()))).toContain(
      `${revisionFile}:1:1:`,
    )
    rmSync(revisionFile, { recursive: true })

    const retry = await client.post('/extract', app.post('a.tsx', source))

    expect(retry.status).toBe(200)
    await instance.flush()
    expect(
      readCoordinatorState(stateFile, '')?.inputs.map((input) => input.source),
    ).toEqual([source])
    expect(readFileSync(revisionFile, 'utf-8')).toBe('1')
  })
})
