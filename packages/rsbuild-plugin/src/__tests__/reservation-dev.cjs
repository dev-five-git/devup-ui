const assert = require('node:assert/strict')
const { addPair, put } = require('./reservation-fixture.cjs')

async function runDev(rsbuild, state) {
  let complete
  const nextCompile = () =>
    new Promise((resolve, reject) => {
      complete = { resolve, reject }
    })
  const first = nextCompile()
  rsbuild.onAfterDevCompile(({ stats }) => {
    if (stats.hasErrors()) complete.reject(new Error(stats.toString()))
    else complete.resolve(stats)
  })
  let server
  try {
    server = await rsbuild.createDevServer()
    const { port } = await server.listen()
    const initial = await first
    assert.equal(
      state.extractions.filter((item) =>
        /[\\/]app[\\/][AB]\.js$/.test(item.name),
      ).length,
      0,
    )
    const assets = initial.toJson({ all: false, assets: true }).assets
    let source = ''
    for (const asset of assets.filter((asset) => asset.name.endsWith('.js'))) {
      const response = await fetch(`http://127.0.0.1:${port}/${asset.name}`, {
        signal: AbortSignal.timeout(15000),
      })
      assert.equal(response.status, 200)
      source += await response.text()
    }
    const match = /var data = ("lazy-compilation-proxy\|(?:[^"\\]|\\.)*")/.exec(
      source,
    )
    assert(match, source)
    const key = JSON.parse(match[1])
    addPair(state.root, state.order)
    put(state.root, 'src/constants.js', 'export const tone = "crimson"')
    const route = nextCompile()
    const response = await fetch(
      `http://127.0.0.1:${port}/_rspack/lazy/trigger`,
      {
        method: 'POST',
        body: key,
        headers: { 'content-type': 'text/plain' },
        signal: AbortSignal.timeout(15000),
      },
    )
    assert.equal(response.status, 200)
    await response.text()
    await route
    state.events.push('lazy-route-complete')
    return { key, initialHash: initial.hash }
  } finally {
    if (server) await server.close()
  }
}

module.exports = { runDev }
