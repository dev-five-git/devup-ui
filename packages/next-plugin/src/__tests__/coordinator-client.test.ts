import { randomUUID } from 'node:crypto'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import type { IncomingMessage, Server, ServerResponse } from 'node:http'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import {
  CoordinatorRequestError,
  requestCoordinator,
} from '../coordinator-client'
import {
  formatPortFile,
  parsePortFile,
  publishPortFile,
} from '../coordinator-port'

const servers: Server[] = []
const dirs: string[] = []
function fixture(prefix = 'devup-client-') {
  const dir = mkdtempSync(join(tmpdir(), prefix))
  dirs.push(dir)
  return {
    portFile: join(dir, 'endpoint'),
    identity: { project: dir, token: randomUUID() },
    resourcePath: join(dir, 'App.tsx'),
    path: '/css',
    timeoutMs: 250,
  }
}
async function listen(
  handler: (req: IncomingMessage, res: ServerResponse) => void,
) {
  const server = createServer(handler)
  servers.push(server)
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
  const address = server.address()
  if (!address || typeof address === 'string')
    throw new Error('Expected TCP server')
  return { server, port: address.port }
}
async function healthy(
  options: ReturnType<typeof fixture>,
  handler: (req: IncomingMessage, res: ServerResponse) => void,
) {
  const endpoint = await listen((req, res) => {
    expect(req.headers['x-devup-project']).toBe(
      encodeURIComponent(options.identity.project),
    )
    expect(req.headers['x-devup-token']).toBe(options.identity.token)
    if (req.url === '/health')
      res.end(formatPortFile(endpoint.port, process.pid, options.identity))
    else handler(req, res)
  })
  publishPortFile(
    options.portFile,
    parsePortFile(descriptor(endpoint.port, options)),
  )
  return endpoint
}
function descriptor(port: number, options: ReturnType<typeof fixture>) {
  return formatPortFile(port, process.pid, options.identity)
}
afterEach(async () => {
  for (const server of servers.splice(0)) {
    server.closeAllConnections()
    await new Promise<void>((resolve) => server.close(() => resolve()))
  }
  for (const dir of dirs.splice(0))
    rmSync(dir, { recursive: true, force: true })
})

describe('deadline coordinator transport', () => {
  it('uses separate endpoints and encoded headers for each app including Unicode roots', async () => {
    const first = fixture('테스트-app-'),
      second = fixture()
    await healthy(first, (_req, res) => res.end('FIRST'))
    await healthy(second, (_req, res) => res.end('SECOND'))
    expect(await requestCoordinator(first)).toBe('FIRST')
    expect(await requestCoordinator(second)).toBe('SECOND')
  })
  it('rereads a replaced endpoint on the next operation', async () => {
    const options = fixture()
    await healthy(options, (_req, res) => res.end('OLD'))
    expect(await requestCoordinator(options)).toBe('OLD')
    await healthy(options, (_req, res) => res.end('NEW'))
    expect(await requestCoordinator(options)).toBe('NEW')
  })
  it('recovers within the same invocation after a stale closed port', async () => {
    const options = fixture()
    const stale = await healthy(options, (_req, res) => res.end('OLD'))
    await new Promise<void>((resolve) => stale.server.close(() => resolve()))
    const pending = requestCoordinator({
      ...options,
      method: 'POST' as const,
      path: '/extract',
      body: '{}',
    })
    await healthy(options, (_req, res) => res.end('RECOVERED'))
    expect(await pending).toBe('RECOVERED')
  })
  it('waits for startup publication within the same deadline', async () => {
    const options = fixture()
    const pending = requestCoordinator(options)
    await healthy(options, (_req, res) => res.end('STARTED'))
    expect(await pending).toBe('STARTED')
  })
  it.each(['health', 'response', 'stream', 'missing'])(
    'bounds %s hangs and reports the source location',
    async (phase) => {
      const options = { ...fixture(), timeoutMs: 35 }
      if (phase === 'health') {
        const endpoint = await listen(() => {})
        writeFileSync(options.portFile, descriptor(endpoint.port, options))
      } else if (phase !== 'missing')
        await healthy(options, (_req, res) => {
          if (phase === 'stream') res.write('partial')
        })
      const start = performance.now()
      await expect(requestCoordinator(options)).rejects.toThrow(
        `${options.resourcePath}:1:1:`,
      )
      expect(performance.now() - start).toBeLessThan(1000)
    },
  )
  it.each(['project', 'token'])(
    'rejects a cross-wired %s before dispatch',
    async (field) => {
      const options = fixture()
      let dispatched = 0
      await healthy(options, (_req, res) => {
        dispatched++
        res.end('WRONG')
      })
      const identity =
        field === 'project'
          ? {
              ...options.identity,
              project: join(options.identity.project, 'other'),
            }
          : { ...options.identity, token: randomUUID() }
      await expect(
        requestCoordinator({ ...options, identity }),
      ).rejects.toThrow('identity mismatch')
      expect(dispatched).toBe(0)
    },
  )
  it.each(['port', 'pid', 'project', 'token'] as const)(
    'rejects mismatched health %s',
    async (field) => {
      const options = fixture()
      const endpoint = await listen((_req, res) => {
        const info = parsePortFile(descriptor(endpoint.port, options))
        res.end(
          JSON.stringify({
            ...info,
            [field]: {
              port: 1,
              pid: process.pid + 1,
              project: join(info.project, 'other'),
              token: randomUUID(),
            }[field],
          }),
        )
      })
      writeFileSync(options.portFile, descriptor(endpoint.port, options))
      await expect(requestCoordinator(options)).rejects.toThrow(
        'health ownership mismatch',
      )
    },
  )
  it('pins descriptor identity when optional expected identity is absent', async () => {
    const options = fixture()
    await healthy(options, (_req, res) => res.end('CSS'))
    const operation = { ...options, identity: undefined, timeoutMs: undefined }
    expect(await requestCoordinator(operation)).toBe('CSS')
  })
  it('does not retry ambiguous POST response loss', async () => {
    const options = fixture()
    let posts = 0
    await healthy(options, (req) => {
      posts++
      req.socket.destroy()
    })
    await expect(
      requestCoordinator({
        ...options,
        method: 'POST',
        path: '/extract',
        body: '{}',
      }),
    ).rejects.toBeInstanceOf(CoordinatorRequestError)
    expect(posts).toBe(1)
  })
  it('retries GET response loss safely', async () => {
    const options = fixture()
    let gets = 0
    await healthy(options, (req, res) => {
      gets++
      if (gets === 1) req.socket.destroy()
      else res.end('RETRIED')
    })
    expect(await requestCoordinator(options)).toBe('RETRIED')
    expect(gets).toBe(2)
  })
  it('rejects a partially received POST response exactly once', async () => {
    const options = fixture()
    let posts = 0
    await healthy(options, (_req, res) => {
      posts++
      res.write('partial')
      res.socket?.destroy()
    })
    await expect(
      requestCoordinator({ ...options, method: 'POST', body: '{}' }),
    ).rejects.toThrow()
    expect(posts).toBe(1)
  })
  it('preserves missing filename details from a CSS failure body', async () => {
    const options = fixture()
    await healthy(options, (_req, res) => {
      res.writeHead(503)
      res.end(JSON.stringify({ error: 'Missing app/page.tsx' }))
    })
    await expect(requestCoordinator(options)).rejects.toThrow(
      'Missing app/page.tsx',
    )
  })
  it.each(['{', ' '.repeat(4097)])(
    'rejects malformed health metadata',
    async (body) => {
      const options = fixture()
      const endpoint = await listen((_req, res) => res.end(body))
      writeFileSync(options.portFile, descriptor(endpoint.port, options))
      await expect(requestCoordinator(options)).rejects.toThrow()
    },
  )
  it('reports malformed endpoint metadata without dispatch', async () => {
    const options = fixture()
    writeFileSync(options.portFile, 'broken')
    await expect(requestCoordinator(options)).rejects.toThrow(options.portFile)
  })
  it('rejects already exhausted deadlines without network traffic', async () => {
    const options = fixture()
    await expect(
      requestCoordinator({ ...options, timeoutMs: 0 }),
    ).rejects.toThrow()
    expect(new CoordinatorRequestError(options, 'reason').message).toContain(
      'reason',
    )
  })
})
