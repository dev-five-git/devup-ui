import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

// The landing app's Vinext, with the repository's Bun patch applied.
const landing = join(import.meta.dir, '../../../../apps/landing')
const prerenderEntry = Bun.resolveSync(
  'vinext/internal/build/run-prerender',
  landing,
)

// `vinext build` runs in Node, so the helper does too; its fetch and HTTP
// server are the ones whose keep-alive reuse loses pages.
const child = Bun.spawnSync(
  [
    'node',
    join(import.meta.dir, 'vinext-prerender-transport.helper.mjs'),
    Bun.resolveSync('vinext/server/prod-server', landing),
    join(dirname(prerenderEntry), 'prerender.js'),
  ],
  { stdout: 'pipe', stderr: 'pipe', timeout: 60000 },
)
const report: unknown = JSON.parse(
  child.stdout.toString().trim().split('\n').at(-1) ?? 'null',
)

it('runs the published prerender code to completion', () => {
  expect({
    exitCode: child.exitCode,
    stderr: child.stderr.toString(),
  }).toMatchObject({ exitCode: 0 })
  expect(report).toMatchObject({ idleTimeout: expect.any(Number) })
})

it('serves each build-time prerender request on its own connection', () => {
  expect(report).toMatchObject({
    shipped: {
      maxRequestsPerSocket: 1,
      connection: 'close',
      routes: ['/victim:rendered', '/dynamic:skipped:dynamic', '/404:rendered'],
      error: null,
      html: true,
      flight: true,
      written: ['victim.html', 'victim.txt'],
    },
  })
})

it('keeps Node keep-alive defaults on servers that are not for prerendering', () => {
  expect(report).toMatchObject({ productionMaxRequestsPerSocket: 0 })
})

it('never drops a page silently when the server closes a reused connection', () => {
  // Node 24 resets the reused connection here; newer Node versions render it.
  expect(report).toMatchObject({
    reuse: {
      maxRequestsPerSocket: 1,
      connection: 'keep-alive',
      victim: expect.stringMatching(/^(rendered|failed-closed)$/),
    },
  })
})

it('fails the export naming every route and keeping the original error when a request gets no response', () => {
  expect(report).toMatchObject({
    reset: {
      routes: null,
      error: {
        name: 'AggregateError',
        message:
          '[vinext] Static export failed: 2 prerender requests failed before a response was received.\n' +
          '  /dynamic: fetch failed (ECONNRESET)\n' +
          '  /victim: fetch failed (ECONNRESET)',
        causes: [
          { name: 'TypeError', message: 'fetch failed' },
          { code: 'ECONNRESET' },
        ],
        errors: 2,
      },
      written: [],
    },
  })
})
