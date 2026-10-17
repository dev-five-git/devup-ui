import { type ChildProcess, spawn } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { existsSync } from 'node:fs'
import { request } from 'node:http'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import {
  connect,
  createTestApp,
  failure,
  ownership,
  removeTestApps,
  type TestApp,
} from './coordinator-app'

const fixture = join(import.meta.dir, 'fixtures', 'coordinator-process.ts')
const children: ChildProcess[] = []

function launch(app: TestApp, mode = ''): Promise<ChildProcess> {
  const child = spawn(
    process.execPath,
    [fixture, app.root, app.identity.token, mode],
    {
      stdio: ['pipe', 'pipe', 'inherit'],
    },
  )
  children.push(child)
  return new Promise((resolve, reject) => {
    child.stdout?.on('data', (chunk: Buffer) => {
      if (chunk.toString().includes('ready')) resolve(child)
    })
    child.on('exit', (code) =>
      reject(new Error(`coordinator process exited: ${code}`)),
    )
  })
}

function stopped(child: ChildProcess): Promise<void> {
  return new Promise((resolve) => {
    if (child.exitCode !== null || child.signalCode !== null) {
      resolve()
      return
    }
    child.once('exit', () => resolve())
    child.kill()
  })
}

const box = (bg: string) =>
  `import { Box } from '@devup-ui/react'\nexport const C = () => <Box bg="${bg}" p={4} />\n`

afterEach(async () => {
  await Promise.all(children.splice(0).map(stopped))
  removeTestApps()
})

describe('apps in separate processes', () => {
  it('exits after synchronous close while an extraction body is incomplete', async () => {
    const app = createTestApp()
    const child = await launch(app, 'shutdown')
    const client = connect(app.portFile, app.identity)
    const connected = Promise.withResolvers<void>()
    const failed = Promise.withResolvers<Error>()
    const exited = Promise.withResolvers<number | null>()
    child.once('exit', exited.resolve)
    const req = request({
      hostname: '127.0.0.1',
      port: client.info.port,
      path: '/extract',
      method: 'POST',
      agent: false,
      headers: { ...ownership(app.identity), 'content-length': '100000' },
    })
    req.on('error', failed.resolve)
    req.on('socket', (socket) => socket.once('connect', connected.resolve))
    req.write('{')
    await connected.promise

    child.stdin?.end('close')

    expect(await failed.promise).toBeInstanceOf(Error)
    expect(await exited.promise).toBe(0)
    expect(existsSync(app.portFile)).toBe(false)
  }, 3000)

  it('each serve their own engine and survive the other one dying', async () => {
    const first = createTestApp()
    const second = createTestApp()
    first.write('src/a.tsx', box('red'))
    second.write('src/a.tsx', box('blue'))
    const [firstProcess, secondProcess] = await Promise.all([
      launch(first),
      launch(second),
    ])
    const one = connect(first.portFile, first.identity)
    const two = connect(second.portFile, { ...second.identity })

    await one.post('/extract', first.post('src/a.tsx'))
    await two.post('/extract', second.post('src/a.tsx'))
    const firstCss = (await one.get('/css?fileNum=0')).body
    const secondCss = (await two.get('/css?fileNum=0')).body
    await stopped(firstProcess)

    expect(one.info.pid).toBe(firstProcess.pid ?? -1)
    expect(two.info.pid).toBe(secondProcess.pid ?? -1)
    expect(one.info.pid).not.toBe(process.pid)
    expect(one.info.port).not.toBe(two.info.port)
    expect(firstCss).toContain('background:red')
    expect(firstCss).not.toContain('background:blue')
    expect(secondCss).toContain('background:blue')
    expect(secondCss).not.toContain('background:red')
    expect((await two.get('/health')).status).toBe(200)
    expect(await failure(one.get('/health'))).toBeInstanceOf(Error)
    expect(existsSync(second.portFile)).toBe(true)
  })

  it("refuses a client that holds another app's token", async () => {
    const app = createTestApp()
    await launch(app)
    const stranger = connect(app.portFile, {
      project: app.identity.project,
      token: randomUUID(),
    })

    const reply = await stranger.get('/health')

    expect(reply.status).toBe(403)
  })
})
