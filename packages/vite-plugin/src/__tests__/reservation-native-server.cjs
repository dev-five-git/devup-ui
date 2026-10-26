const assert = require('node:assert/strict')
const { revision } = require('./reservation-native-revision.cjs')

/** @typedef {ReturnType<typeof import('./reservation-native-observer.cjs').observe>} Telemetry */
/** @param {import('vite').InlineConfig} config @param {{scenario:string,root:string,order:string,paths:readonly string[],base:string,telemetry:Telemetry,http:Record<string,string>,details:Record<string,unknown>,ready:Promise<unknown>,nativeUpdate:Promise<number>,checkpoint:(phase:string)=>void}} context */
async function runServer(config, context) {
  const { scenario, root, order, paths, telemetry, http, details, checkpoint } =
    context
  const { createServer } = await import('vite')
  if (scenario === 'serve-ssr')
    config.environments = {
      ssr: { consumer: 'server', build: { ssr: 'server.tsx' } },
      client: { consumer: 'client' },
    }
  const server = await createServer(config)
  try {
    await server.listen()
    await context.ready
    checkpoint('native-ready')
    const address = server.httpServer?.address()
    assert(address && typeof address === 'object')
    const base = `http://127.0.0.1:${address.port}`
    if (scenario === 'serve-ssr') {
      const environment = server.environments.ssr
      assert(environment)
      const result = await environment.transformRequest('/server.tsx')
      assert(result)
      http.ssr = result.code
      details.beforeHtml = telemetry.snapshot()
    }
    if (['hmr', 'retained', 'history', 'linked'].includes(scenario)) {
      await revision({
        server,
        base,
        root,
        scenario,
        order,
        telemetry,
        http,
        details,
        nativeUpdate: context.nativeUpdate,
        checkpoint,
      })
    } else {
      const responses = await Promise.all(
        paths.map(async (path) => {
          const response = await fetch(base + path)
          assert.equal(response.status, 200)
          return [path, await response.text()]
        }),
      )
      Object.assign(http, Object.fromEntries(responses))
      const response = await fetch(base + context.base)
      assert.equal(response.status, 200)
      http.html = await response.text()
    }
    checkpoint('modules-http-complete')
    for (const path of new Set(
      Object.values(http).flatMap((code) =>
        [
          ...code.matchAll(
            /['"](\/[^'"\n]*devup-ui(?:-\d+)?\.css[^'"\n]*)['"]/g,
          ),
        ].flatMap((match) => (match[1] ? [match[1]] : [])),
      ),
    )) {
      details.pendingCssRequest = path + '&direct'
      checkpoint('css-http-enter')
      const response = await fetch(base + path + '&direct')
      assert.equal(response.status, 200)
      http[path] = await response.text()
      checkpoint('css-http-return')
    }
    checkpoint('outcome-before-close')
  } catch (error) {
    details.error =
      error instanceof Error
        ? { message: error.message, stack: error.stack }
        : String(error)
    checkpoint('error-before-close')
    throw error
  } finally {
    checkpoint('close-enter')
    await server.close()
    details.closed = true
    checkpoint('close-exit')
  }
}
module.exports = { runServer }
