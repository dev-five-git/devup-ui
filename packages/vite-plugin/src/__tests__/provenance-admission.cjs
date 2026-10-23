const assert = require('node:assert/strict')
const { join } = require('node:path')
const { provenanceFixture, observe, wasm } = require('./provenance-fixture.cjs')
const mode = process.argv[2]
const root = process.argv[3]
assert(root)

async function main() {
  const telemetry = observe()
  const state = provenanceFixture(root)
  const { plugin, transform } = state
  state.write(
    'settings.json',
    '{"theme":{"colors":{"default":{"primary":"red"}}}}',
  )
  state.write('devup.json', '{"extends":["./settings.json"]}')
  await plugin.configResolved(state.config)
  const { createServer } = await import('vite')
  const server = await createServer({
    root,
    configFile: false,
    server: { watch: null },
    logLevel: 'silent',
  })
  plugin.configureServer(server)
  const notify = async (/** @type {string} */ file) => {
    server.watcher.emit('all', 'change', file)
    switch (mode) {
      case 'watchChange':
        await plugin.watchChange(file)
        break
      case 'hotUpdate':
        await plugin.hotUpdate.call(
          { environment: server.environments.client },
          { file, modules: [], timestamp: 51 },
        )
        break
      case 'handleHotUpdate':
        await plugin.handleHotUpdate({
          file,
          modules: [],
          timestamp: 51,
          server,
        })
        break
      default:
        throw new TypeError('Unknown admission mode ' + mode)
    }
  }
  try {
    // Given: an eligible flight and an actual JSON config dependency.
    await transform()
    const count = telemetry.extractions.length
    const reservations = telemetry.reservations.length
    // When: arbitrary unrelated JSON passes native admission and the selected hook.
    state.write('arbitrary.json', '{"value":1}')
    await notify(join(root, 'arbitrary.json'))
    await transform()
    // Then: no preparation or extraction is repeated.
    assert.equal(
      telemetry.reservations.length,
      reservations,
      'unrelated JSON must not advance actual reservation',
    )
    assert.equal(
      telemetry.extractions.length,
      count,
      'unrelated JSON repeat must reuse',
    )
    // Given / When: the same suffix belongs to an actual inherited dependency.
    state.write(
      'settings.json',
      '{"theme":{"colors":{"default":{"primary":"blue"}}}}',
    )
    await notify(join(root, 'settings.json'))
    await transform()
    // Then: real theme output changes and the old flight is invalid.
    assert.equal(telemetry.extractions.length, count + 1)
    assert.match(wasm.getCss(null, false), /--primary:blue/)
  } finally {
    await plugin.closeBundle()
    await server.close()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
