import * as fs from 'node:fs'
import * as fsp from 'node:fs/promises'
import { join } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import { resetCoordinator, startCoordinator } from '../coordinator'
import { readCoordinatorState } from '../state'
import {
  connect,
  failure,
  http,
  instrument,
  removeTestApps,
} from './coordinator-app'
import { acceptedRequests, barrier } from './coordinator-barrier-fixture'

const source =
  'import { Box } from "@devup-ui/react"; export const A = <Box bg="red" />'

afterEach(() => {
  resetCoordinator()
  removeTestApps()
})

describe('deferred ownership transport', () => {
  it('answers authenticated health before preparation without snapshots or watches', async () => {
    // Given pending preparation with durable output and real watches.
    const { app, pending, entered, handle } = barrier()
    const watching = spyOn(fs, 'watch')
    const stateFile = join(app.root, 'df', 'snapshot.json')
    const revisionFile = join(app.root, 'df', 'revision')
    try {
      await handle.ready
      await entered
      const client = connect(app.portFile, app.identity)

      // When the ownership transport receives a health request.
      const health = await client.get('/health')

      // Then ownership is usable before Core side effects begin.
      expect(health.status).toBe(200)
      expect((await http(client.info.port, 'GET', '/health')).status).toBe(403)
      expect(fs.existsSync(stateFile)).toBe(false)
      expect(fs.existsSync(revisionFile)).toBe(false)
      expect(watching).not.toHaveBeenCalled()
      app.write('src/a.tsx', source)
      pending.resolve(
        app.options({
          watch: true,
          sourceRoots: ['src'],
          stateFile,
          revisionFile,
        }),
      )
      await handle.prepared
      expect(readCoordinatorState(stateFile, '')?.inputs).toEqual([])
      expect(fs.existsSync(revisionFile)).toBe(true)
      expect(watching).toHaveBeenCalled()
    } finally {
      watching.mockRestore()
    }
  })

  it('holds accepted extraction and CSS until complete options and startup arrive', async () => {
    // Given two accepted work requests and a deliberately unresolved compiler.
    const { app, pending, handle } = barrier()
    const { engine, extractions } = instrument(app.engine())
    app.write('src/a.tsx', source)
    await handle.ready
    const client = connect(app.portFile, app.identity)
    const observed = acceptedRequests(2)
    let replies = 0
    try {
      const extracted = client
        .post('/extract', app.post('src/a.tsx'))
        .then((reply) => {
          replies += 1
          return reply
        })
      const css = client
        .get('/css?fileNum=0&waitForIdle=true')
        .then((reply) => {
          replies += 1
          return reply
        })
      await observed.accepted
      await client.get('/health')

      // When complete options are withheld and subsequently supplied.
      expect(extractions).toEqual([])
      expect(replies).toBe(0)
      pending.resolve(
        app.options({ wasm: engine, expectedBaseFiles: ['src/a.tsx'] }),
      )

      // Then only the prepared engine serves extraction and complete CSS.
      expect((await extracted).status).toBe(200)
      expect((await css).body).toContain('background:red')
      expect(extractions).toEqual(['map:src/a.tsx'])
    } finally {
      observed.restore()
    }
  })

  it.each([false, true])(
    'retains a located preparation failure at live dev/prod endpoints (watch=%s)',
    async (watch) => {
      // Given accepted dev or production requests behind pending preparation.
      const { app, pending, handle } = barrier()
      const options = app.options({ watch })
      const error = new Error('src/page.mdx:4:7: required compiler failed')
      await handle.ready
      const client = connect(app.portFile, app.identity)
      const observed = acceptedRequests(2)
      const failed = failure(handle.prepared)
      try {
        const extract = client.post('/extract', app.post('src/a.tsx', source))
        const css = client.get('/css?waitForIdle=true')
        await observed.accepted

        // When the configured compiler rejects with a located cause.
        pending.reject(error)

        // Then live endpoints and drain retain that exact cause.
        expect(await failed).toBe(error)
        for (const reply of [
          await extract,
          await css,
          await client.get('/css'),
        ]) {
          expect(reply.status).toBe(500)
          expect(JSON.parse(reply.body).error).toBe(error.message)
        }
        expect((await client.get('/health')).status).toBe(200)
        expect(await failure(handle.drain())).toBe(error)
        expect(fs.existsSync(options.coordinatorPortFile)).toBe(false)
      } finally {
        observed.restore()
      }
    },
  )

  it('keeps an unrelated app usable while sharing one pending owner across handles', async () => {
    // Given shared pending ownership and a separately prepared app.
    const { app, pending, handle } = barrier()
    const second = startCoordinator({
      projectRoot: app.root,
      identity: app.identity,
      coordinatorPortFile: app.portFile,
      prepare: () => {
        throw new Error('duplicate preparation')
      },
    })
    const other = barrier()
    other.pending.resolve(other.app.options({ watch: true }))
    await Promise.all([handle.ready, other.handle.ready, other.handle.prepared])
    const client = connect(app.portFile, app.identity)

    // When only one shared reference is released.
    handle.close()
    const otherReply = await connect(
      other.app.portFile,
      other.app.identity,
    ).post('/extract', other.app.post('a.tsx', source))

    // Then neither app loses ownership and preparation remains shared.
    expect(otherReply.status).toBe(200)
    expect((await client.get('/health')).status).toBe(200)
    expect(handle.prepared).toBe(second.prepared)
    pending.resolve(app.options({ prewarmedFiles: [] }))
    await second.prepared
    await second.drain()
    expect(fs.existsSync(app.portFile)).toBe(false)
  })

  it('gates work and watches behind the actual startup snapshot', async () => {
    // Given complete options whose real startup rename is held back.
    const { app, pending, handle } = barrier()
    const stateFile = join(app.root, 'df', 'snapshot.json')
    const started = Promise.withResolvers<void>()
    const release = Promise.withResolvers<void>()
    const rename = fsp.rename
    const writing = spyOn(fsp, 'rename').mockImplementation(async (...args) => {
      started.resolve()
      await release.promise
      return rename(...args)
    })
    const watching = spyOn(fs, 'watch')
    const { engine, extractions } = instrument(app.engine())
    try {
      await handle.ready
      pending.resolve(
        app.options({
          wasm: engine,
          watch: true,
          sourceRoots: ['src'],
          stateFile,
        }),
      )
      await started.promise
      const client = connect(app.portFile, app.identity)
      const observed = acceptedRequests(1)
      try {
        const extraction = client.post('/extract', app.post('a.tsx', source))
        await observed.accepted

        // When extraction arrives before startup durability.
        expect(extractions).toEqual([])
        expect(watching).not.toHaveBeenCalled()
        release.resolve()

        // Then work and watches begin only after the write is released.
        expect((await extraction).status).toBe(200)
        await handle.prepared
        expect(watching).toHaveBeenCalled()
      } finally {
        observed.restore()
      }
    } finally {
      release.resolve()
      writing.mockRestore()
      watching.mockRestore()
    }
  })

  it('retains a startup failure at the published endpoint', async () => {
    // Given a published transport with an unwritable snapshot destination.
    const { app, pending, handle } = barrier()
    const stateFile = join(app.root, 'df', 'state.json')
    fs.mkdirSync(stateFile)
    await handle.ready
    const client = connect(app.portFile, app.identity)

    // When complete options trigger the actual startup write.
    pending.resolve(app.options({ stateFile }))
    const error = await failure(handle.prepared)

    // Then ownership stays live and every work route exposes the startup cause.
    if (!(error instanceof Error)) throw error
    expect(error.message).toContain(`${stateFile}:1:1:`)
    expect(
      JSON.parse((await client.get('/css?waitForIdle=true')).body).error,
    ).toBe(error.message)
    expect(
      JSON.parse(
        (await client.post('/extract', app.post('a.tsx', source))).body,
      ).error,
    ).toBe(error.message)
    expect((await client.get('/health')).status).toBe(200)
    expect(await failure(handle.drain())).toBeInstanceOf(Error)
    expect(fs.existsSync(app.portFile)).toBe(false)
  })
})
