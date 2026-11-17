import * as fs from 'node:fs'
import { mkdtempSync, rmSync, unlinkSync, writeFileSync } from 'node:fs'
import { createServer, type Server } from 'node:http'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import {
  callCoordinator,
  type CoordinatorCall,
  type CoordinatorReply,
  resetCoordinatorClient,
  setCoordinatorResponseTimeoutForTesting,
} from '../coordinator-client'
import { formatPortFile } from '../coordinator-port'

const dirs: string[] = []
const servers: Server[] = []

function tmp(): string {
  const dir = mkdtempSync(join(tmpdir(), 'devup-client-'))
  dirs.push(dir)
  return dir
}

/** A coordinator stand-in that answers every request with `body`. */
async function serve(body: string): Promise<number> {
  const server = createServer((req, res) => {
    const chunks: Buffer[] = []
    req.on('data', (chunk: Buffer) => chunks.push(chunk))
    req.on('end', () => {
      res.writeHead(200, { 'Content-Type': 'text/plain' })
      res.end(`${body}:${req.method}:${req.url}:${Buffer.concat(chunks)}`)
    })
  })
  servers.push(server)
  await new Promise<void>((done) => server.listen(0, '127.0.0.1', done))
  return (server.address() as { port: number }).port
}

/** A port that nothing listens on: bound once, then released. */
async function deadPort(): Promise<number> {
  const server = createServer()
  await new Promise<void>((done) => server.listen(0, '127.0.0.1', done))
  const port = (server.address() as { port: number }).port
  await new Promise<void>((done) => server.close(() => done()))
  return port
}

function call(
  portFile: string,
  request: CoordinatorCall = { path: '/css', method: 'GET' },
): Promise<CoordinatorReply> {
  return new Promise((done, fail) =>
    callCoordinator(portFile, request, (error, reply) =>
      reply === undefined ? fail(error) : done(reply),
    ),
  )
}

afterEach(async () => {
  resetCoordinatorClient()
  for (const server of servers.splice(0)) {
    server.closeAllConnections()
    await new Promise<void>((done) => server.close(() => done()))
  }
  for (const dir of dirs.splice(0))
    rmSync(dir, { recursive: true, force: true })
})

describe('callCoordinator', () => {
  it('keeps a separate port for each port file', async () => {
    const dir = tmp()
    const first = join(dir, 'first.port')
    const second = join(dir, 'second.port')
    writeFileSync(first, formatPortFile(await serve('FIRST')))
    writeFileSync(second, formatPortFile(await serve('SECOND')))

    expect((await call(first)).body).toBe('FIRST:GET:/css:')
    expect((await call(second)).body).toBe('SECOND:GET:/css:')
  })

  it('sends a POST body with its status', async () => {
    const dir = tmp()
    const portFile = join(dir, 'coordinator.port')
    writeFileSync(portFile, formatPortFile(await serve('APP')))

    expect(
      await call(portFile, { path: '/extract', method: 'POST', body: '{}' }),
    ).toEqual({ statusCode: 200, body: 'APP:POST:/extract:{}' })
  })

  it('follows a coordinator that restarted on a new port', async () => {
    const dir = tmp()
    const portFile = join(dir, 'coordinator.port')
    writeFileSync(portFile, formatPortFile(await serve('OLD')))
    expect((await call(portFile)).body).toBe('OLD:GET:/css:')

    // The old coordinator exits and its replacement publishes a new port.
    for (const server of servers.splice(0)) {
      server.closeAllConnections()
      await new Promise<void>((done) => server.close(() => done()))
    }
    writeFileSync(portFile, formatPortFile(await serve('NEW')))

    expect((await call(portFile)).body).toBe('NEW:GET:/css:')
  })

  it('reports a coordinator that still names the port that refused', async () => {
    const dir = tmp()
    const portFile = join(dir, 'coordinator.port')
    const port = await deadPort()
    writeFileSync(portFile, formatPortFile(port))

    await expect(call(portFile)).rejects.toThrow(
      `Coordinator unreachable at 127.0.0.1:${port}`,
    )
  })

  it('reports a restarted coordinator whose new port refuses too', async () => {
    const dir = tmp()
    const portFile = join(dir, 'coordinator.port')
    writeFileSync(portFile, formatPortFile(await serve('OLD')))
    await call(portFile)
    for (const server of servers.splice(0)) {
      server.closeAllConnections()
      await new Promise<void>((done) => server.close(() => done()))
    }
    const next = await deadPort()
    writeFileSync(portFile, formatPortFile(next))

    await expect(call(portFile)).rejects.toThrow(
      `Coordinator unreachable at 127.0.0.1:${next}`,
    )
  })

  it('reports a coordinator whose port file disappears after it refused', async () => {
    const dir = tmp()
    const portFile = join(dir, 'coordinator.port')
    writeFileSync(portFile, formatPortFile(await serve('OLD')))
    await call(portFile)
    for (const server of servers.splice(0)) {
      server.closeAllConnections()
      await new Promise<void>((done) => server.close(() => done()))
    }
    unlinkSync(portFile)

    await expect(call(portFile)).rejects.toThrow('Coordinator unreachable at')
  })

  it('waits for a port file that is not completely written yet', async () => {
    const dir = tmp()
    const portFile = join(dir, 'coordinator.port')
    writeFileSync(portFile, '')
    const port = await serve('LATE')
    setTimeout(() => writeFileSync(portFile, formatPortFile(port)), 100)

    expect((await call(portFile)).body).toBe('LATE:GET:/css:')
  })

  it('reports a port file that never names a valid port', async () => {
    const dir = tmp()
    const portFile = join(dir, 'coordinator.port')
    writeFileSync(portFile, '70000')

    await expect(call(portFile)).rejects.toThrow(
      `Coordinator port file ${portFile} names no valid port.`,
    )
  })

  it('reports a port file that cannot be read', async () => {
    const dir = tmp()
    const portFile = join(dir, 'coordinator.port')
    writeFileSync(portFile, '1234')
    const read = spyOn(fs, 'readFileSync').mockImplementation(() => {
      throw 'EACCES'
    })
    try {
      await expect(call(portFile)).rejects.toThrow('EACCES')
    } finally {
      read.mockRestore()
    }
  })

  it('gives up on a coordinator that accepts the request and never answers', async () => {
    const dir = tmp()
    const portFile = join(dir, 'coordinator.port')
    const server = createServer(() => {})
    servers.push(server)
    await new Promise<void>((done) => server.listen(0, '127.0.0.1', done))
    const port = (server.address() as { port: number }).port
    writeFileSync(portFile, formatPortFile(port))
    setCoordinatorResponseTimeoutForTesting(50)

    await expect(call(portFile)).rejects.toThrow(
      `Coordinator at 127.0.0.1:${port} (port file ${portFile}) sent no response for 0.05s.`,
    )
  })
})
