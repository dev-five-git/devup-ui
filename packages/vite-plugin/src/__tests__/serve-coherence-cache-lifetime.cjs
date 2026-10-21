const assert = require('node:assert/strict')
const { join } = require('node:path')
const {
  fixture,
  gate,
  observe,
  wasm,
} = require('./serve-coherence-cache-fixture.cjs')
const mode = process.argv[2]
const root = process.argv[3]
assert(root)
const telemetry = observe()

async function main() {
  // Given: a real owned serve session, with an actual sheet write held pending.
  const state = fixture(root)
  const { plugin, transform, source } = state
  await plugin.configResolved(state.config)
  const suspended = gate('writeFile', join(root, 'df/css/devup-ui.css'))
  const first = transform()
  try {
    await suspended.entered
    const count = telemetry.extractions.length
    // When: one lifetime or semantic boundary occurs before that write completes.
    switch (mode) {
      case 'flight': {
        const second = transform()
        suspended.release()
        assert.deepEqual(await second, await first)
        assert.equal(
          telemetry.extractions.length,
          count,
          'pending identical calls must join one flight',
        )
        break
      }
      case 'reconfigure': {
        await plugin.configResolved(state.config)
        await transform()
        suspended.release()
        await first
        await transform()
        assert.equal(
          telemetry.extractions.length,
          count + 1,
          'old completion must not replace the new session flight',
        )
        break
      }
      case 'refresh': {
        state.write(
          'devup.json',
          '{"theme":{"colors":{"default":{"primary":"blue"}}}}',
        )
        await plugin.watchChange(join(root, 'devup.json'))
        suspended.release()
        await first
        await transform()
        assert.equal(
          telemetry.extractions.length,
          count + 1,
          'pre-refresh capture cannot be retained after old write completes',
        )
        assert.match(wasm.getCss(null, false), /--primary:blue/)
        break
      }
      case 'close': {
        const closed = plugin.closeBundle()
        suspended.release()
        await first
        await closed
        assert.equal(
          await transform(),
          undefined,
          'serve close must remain terminal',
        )
        break
      }
      default:
        throw new TypeError('Unknown lifetime mode ' + mode)
    }
    // Then: only a current successful flight may serve the next identical call.
    if (mode === 'flight') {
      await transform(source)
      assert.equal(telemetry.extractions.length, count)
    }
  } finally {
    suspended.restore()
    await first
    await plugin.closeBundle()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
