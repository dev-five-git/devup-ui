// Runs Vinext's published prerenderApp in Node, the runtime of `vinext build`,
// against servers started by Vinext's own startProdServer, and prints a JSON
// summary for vinext-prerender-transport.test.ts.
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const [prodServerPath, prerenderPath] = process.argv.slice(2)
const { startProdServer } = await import(pathToFileURL(prodServerPath).href)
const { prerenderApp } = await import(pathToFileURL(prerenderPath).href)

const flight = '0:["$","main",null,{"children":"victim"}]\n'
const html = `<!doctype html><html><body><main>victim</main><script>self.__VINEXT_RSC_CHUNKS__=self.__VINEXT_RSC_CHUNKS__||[];self.__VINEXT_RSC_CHUNKS__.push(${JSON.stringify(flight)});</script><script>self.__VINEXT_RSC_DONE__=true;</script></body></html>`
const entry = `export default async function handler(request) {
  const { pathname } = new URL(request.url)
  if (pathname === '/warm') return new Response('warm')
  if (pathname === '/victim') return new Response(${JSON.stringify(html)}, { headers: { 'content-type': 'text/html; charset=utf-8' } })
  if (pathname === '/dynamic') return new Response('dynamic', { headers: { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' } })
  return new Response('<!doctype html><title>404</title>', { status: 404, headers: { 'content-type': 'text/html; charset=utf-8' } })
}
`

function causeChain(error) {
  const chain = []
  for (let cause = error; cause instanceof Error; cause = cause.cause)
    chain.push({ name: cause.name, message: cause.message, code: cause.code })
  return chain
}

function summarize(outDir, outcome) {
  const victimHtml = join(outDir, 'victim.html')
  const victimFlight = join(outDir, 'victim.txt')
  return {
    routes:
      outcome.routes?.map(({ route, status, reason }) =>
        [route, status, reason].filter(Boolean).join(':'),
      ) ?? null,
    error: outcome.error ?? null,
    html: existsSync(victimHtml) && readFileSync(victimHtml, 'utf8') === html,
    flight:
      existsSync(victimFlight) && readFileSync(victimFlight, 'utf8') === flight,
    written: ['victim.html', 'victim.txt', 'dynamic.html'].filter((file) =>
      existsSync(join(outDir, file)),
    ),
  }
}

function classifyVictim({ routes, error, html, flight, written }) {
  if (error)
    return error.name === 'AggregateError' &&
      error.message.endsWith('\n  /victim: fetch failed (ECONNRESET)') &&
      written.length === 0
      ? 'failed-closed'
      : error.message
  return routes.includes('/victim:rendered') && html && flight
    ? 'rendered'
    : routes.find((route) => route.startsWith('/victim'))
}

function prerender(prodServer, outDir, serverDir, rscBundlePath, routes) {
  return prerenderApp({
    routes,
    metadataRoutes: [],
    outDir,
    serverDir,
    rscBundlePath,
    mode: 'export',
    config: { basePath: '', trailingSlash: false },
    _prodServer: prodServer,
  }).then(
    (result) => summarize(outDir, { routes: result.routes }),
    (error) =>
      summarize(outDir, {
        error: {
          name: error.name,
          message: error.message,
          causes: causeChain(error.cause),
          errors: error.errors?.length ?? 0,
        },
      }),
  )
}

const root = mkdtempSync(join(tmpdir(), 'vinext-prerender-transport-'))
try {
  writeFileSync(join(root, 'package.json'), '{"type":"module"}')
  const serverDir = join(root, 'server')
  mkdirSync(serverDir)
  mkdirSync(join(root, 'client'))
  const rscBundlePath = join(serverDir, 'index.js')
  writeFileSync(rscBundlePath, entry)
  const routes = ['victim', 'dynamic'].map((name) => {
    const pagePath = join(root, `${name}.tsx`)
    writeFileSync(
      pagePath,
      `export default function Page() { return <main>${name}</main> }\n`,
    )
    return {
      pattern: `/${name}`,
      pagePath,
      routePath: null,
      isDynamic: false,
      layouts: [],
      parallelSlots: [],
    }
  })

  const production = await startProdServer({
    port: 0,
    host: '127.0.0.1',
    outDir: root,
    rscEntryPath: rscBundlePath,
    serverDir,
    noCompression: true,
    silent: true,
  })
  const productionMaxRequestsPerSocket = production.server.maxRequestsPerSocket
  production.server.close()

  // "shipped" keeps the server exactly as Vinext starts it for prerendering;
  // "reuse" restores Node's default keep-alive reuse on that same server.
  const servers = {}
  const warm = {}
  for (const name of ['shipped', 'reuse']) {
    const prodServer = await startProdServer({
      port: 0,
      host: '127.0.0.1',
      outDir: root,
      rscEntryPath: rscBundlePath,
      serverDir,
      noCompression: true,
      purpose: 'prerender',
      silent: true,
    })
    warm[name] = {
      maxRequestsPerSocket: prodServer.server.maxRequestsPerSocket,
    }
    if (name === 'reuse') prodServer.server.maxRequestsPerSocket = 0
    const response = await fetch(`http://127.0.0.1:${prodServer.port}/warm`)
    await response.text()
    warm[name].connection = response.headers.get('connection')
    servers[name] = prodServer
  }

  // Queue the prerender requests, then block the event loop past the servers'
  // idle keep-alive timeout, as a long render does during a build.
  const idleTimeout = Math.max(
    ...Object.values(servers).map(
      ({ server }) => server.keepAliveTimeout + server.keepAliveTimeoutBuffer,
    ),
  )
  const [shipped, reuse] = await new Promise((resolve) =>
    setImmediate(() => {
      const pending = ['shipped', 'reuse'].map((name) =>
        prerender(
          servers[name],
          join(root, name),
          serverDir,
          rscBundlePath,
          routes,
        ),
      )
      const start = performance.now()
      while (performance.now() - start < idleTimeout);
      resolve(Promise.all(pending))
    }),
  )
  for (const { server } of Object.values(servers)) {
    server.closeAllConnections()
    server.close()
  }

  // "reset" stands in for a prerender server whose connection fails after the
  // request is sent: every request gets a TCP reset and no response.
  const resetServer = createServer((socket) =>
    socket.once('data', () => socket.resetAndDestroy()),
  )
  await new Promise((resolve) => resetServer.listen(0, '127.0.0.1', resolve))
  const reset = await prerender(
    { server: resetServer, port: resetServer.address().port },
    join(root, 'reset'),
    serverDir,
    rscBundlePath,
    routes,
  )
  resetServer.close()

  process.stdout.write(
    `${JSON.stringify({
      idleTimeout,
      productionMaxRequestsPerSocket,
      shipped: { ...warm.shipped, ...shipped },
      reuse: { ...warm.reuse, victim: classifyVictim(reuse) },
      reset,
    })}\n`,
  )
} finally {
  rmSync(root, { recursive: true, force: true })
}
