const assert = require('node:assert/strict')
const { createServer } = require('node:http')
const { writeFileSync } = require('node:fs')
const { join } = require('node:path')
const { revision } = require('./native-webpack-fixture.cjs')
const { request, surface } = require('./native-webpack-http.cjs')
const { parseEdge } = require('./native-webpack-receipt.ts')
const nextModule = require('next')
// Next declares an ESM default but its installed CJS entry exports the callable itself.
const next = nextModule.default ?? nextModule

const root = process.argv[2] ?? ''
const mode = process.argv[3]
assert(root && (mode === 'production' || mode === 'development'))
process.chdir(root)
delete process.env.TURBOPACK
Object.assign(process.env, { NODE_ENV: mode })
const observer = require('./native-webpack-observer.cjs')
async function run() {
  const app = next({ dir: root, dev: mode === 'development', webpack: true })
  const server = createServer(app.getRequestHandler())
  const scope = new AbortController()
  /** @type {unknown[]} */
  const failures = []
  try {
    await app.prepare()
    await new Promise((resolve, reject) => {
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        server.off('error', reject)
        resolve(undefined)
      })
    })
    const address = server.address()
    assert(address && typeof address !== 'string')
    const base = `http://127.0.0.1:${address.port}/`
    const initial = await surface(base, false, scope.signal)
    const edge = parseEdge(await request(`${base}edge`, scope.signal))
    const old = observer.snapshot()
    let revised
    if (mode === 'development') {
      observer.revise()
      const complete = observer.until(
        (records) =>
          records.some(
            (entry) =>
              entry.kind === 'done' &&
              entry.phase === 'revision' &&
              records.some(
                (item) =>
                  item.kind === 'extract' && item.args[0] === 'src/app/A.js',
              ) &&
              records.some(
                (item) =>
                  item.kind === 'extract' && item.args[0] === 'src/app/B.js',
              ),
          ),
        scope.signal,
      )
      await Promise.all([
        complete,
        Promise.resolve().then(() => revision(root)),
      ])
      revised = await surface(base, true, scope.signal)
      observer.assertReserved()
    }
    writeFileSync(
      join(root, `native-${mode}.json`),
      JSON.stringify({
        initial,
        edge,
        old,
        revised,
        final: observer.snapshot(),
      }),
    )
  } catch (error) {
    failures.push(error)
  } finally {
    scope.abort()
    const deadline = setTimeout(() => {
      console.error(new Error('Native Next cleanup deadline exceeded'))
      process.exit(1)
    }, 15000)
    const closed = await Promise.allSettled([
      new Promise((resolve, reject) => {
        server.closeAllConnections()
        if (!server.listening) {
          resolve(undefined)
          return
        }
        server.close((error) => (error ? reject(error) : resolve(undefined)))
      }),
      Promise.resolve().then(() => app.close()),
    ])
    clearTimeout(deadline)
    writeFileSync(
      join(root, 'native-cleanup.json'),
      JSON.stringify({
        pid: process.pid,
        closed: closed.map((result) => result.status),
        resources: process.getActiveResourcesInfo(),
      }),
    )
    failures.push(
      ...closed.flatMap((result) =>
        result.status === 'rejected' ? [result.reason] : [],
      ),
    )
  }
  if (failures.length > 0)
    throw new AggregateError(failures, 'Native Next scenario or cleanup failed')
}
// This is an isolated test CLI: fulfillment includes the scenario and both closes.
// The external owner drains the entire native tree before returning to assertions.
run().then(
  () => process.exit(0),
  (error) => {
    console.error(error)
    process.exit(1)
  },
)
