const assert = require('node:assert/strict')
const { join } = require('node:path')
const wasm = require('@devup-ui/wasm')
/** @type {typeof import('../../../plugin-utils/src/index')} */
const utils = require('@devup-ui/plugin-utils')
const { observe } = require('./reservation-native-observer.cjs')
const { project, gate } = require('./serve-coherence-async-fixture.cjs')
const mode = process.argv[2]
assert(mode === 'overlap' || mode === 'solo' || mode === 'pending')
const telemetry = observe()

async function main() {
  // Given: a real build lease was closed after producing frozen state.
  const a = mode === 'solo' ? undefined : project()
  if (a) await a.plugin.configResolved(a.config)
  const b = project()
  b.write('devup.json', '{"theme":{"length":{"default":{"size":"73px"}}}}')
  const config = { ...b.config, command: /** @type {const} */ ('build') }
  await b.plugin.configResolved(config)
  const context = { addWatchFile() {} }
  await b.plugin.buildStart.call(context)
  await b.plugin.transform.call(context, b.source, join(b.root, 'src/A.tsx'))
  const maps = [wasm.exportClassMap(), wasm.exportFileMap(), wasm.exportSheet()]
  await b.plugin.closeBundle()
  const ended = utils.getBuildOwnership()
  const count = telemetry.extractions.length
  /** @type {ReturnType<typeof utils.getBuildOwnership> | undefined} */
  let extractionLease
  telemetry.onExtract(() => {
    extractionLease = utils.getBuildOwnership()
  })
  let suspended
  let pending = Promise.resolve()
  let closing
  try {
    // When: another environment/build-watch cycle admits work on the same plugin.
    if (mode === 'pending') {
      suspended = gate('writeFile', join(b.root, 'df/compat.d.ts'))
      pending = b.plugin.configResolved(config)
      const entered = await Promise.race([
        suspended.entered.then(() => true),
        pending.then(() => false),
      ])
      assert(entered, 'later build configuration must be admitted')
      const resumed = utils.getBuildOwnership()
      closing = b.plugin.closeBundle()
      assert.deepEqual(
        utils.getBuildOwnership(),
        resumed,
        'pending resumed B retains its lease',
      )
      suspended.release()
      await pending
      await closing
      assert.equal(utils.getBuildOwnership().running, ended.running)
    }
    const output = await b.plugin.transform.call(
      context,
      b.source,
      join(b.root, 'src/A.tsx'),
    )
    // Then: actual extraction happens under a renewed lease, without resetting history.
    assert(output, 'later build environment transform must not be rejected')
    assert.equal(telemetry.extractions.length, count + 1)
    assert(extractionLease)
    assert.equal(extractionLease.running, ended.running + 1)
    assert(extractionLease.generation > ended.generation)
    assert.deepEqual(
      [wasm.exportClassMap(), wasm.exportFileMap(), wasm.exportSheet()],
      maps,
      'resuming build ownership must not reset frozen maps or sheet',
    )
    await b.plugin.buildStart.call(context)
    assert.deepEqual(
      utils.getBuildOwnership(),
      extractionLease,
      'an active lease must not be acquired twice',
    )
    await b.plugin.closeBundle()
    assert.equal(utils.getBuildOwnership().running, ended.running)
    console.info('serve-coherence-final-complete')
  } finally {
    suspended?.restore()
    await pending
    await closing
    await b.close()
    await a?.close()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
