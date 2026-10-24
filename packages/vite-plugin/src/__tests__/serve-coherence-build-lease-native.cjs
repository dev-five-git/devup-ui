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
  // Given: real resolved production snapshots retain completed state after close.
  const { resolveConfig } = await import('vite')
  const a = mode === 'solo' ? undefined : project()
  if (a) await a.plugin.configResolved(a.config)
  const b = project()
  b.write('devup.json', '{"theme":{"length":{"default":{"size":"73px"}}}}')
  const context = { addWatchFile() {} }
  const id = join(b.root, 'src/A.tsx')
  const configure = (/** @type {ReturnType<typeof project>} */ state) =>
    resolveConfig(
      {
        root: state.root,
        configFile: false,
        logLevel: 'silent',
        build: {
          lib: { entry: join(state.root, 'src/A.tsx'), formats: ['es'] },
        },
      },
      'build',
    )
  const snapshot = () => [
    wasm.exportClassMap(),
    wasm.exportFileMap(),
    wasm.exportSheet(),
  ]
  if (a) {
    const maps = snapshot()
    const ownership = utils.getBuildOwnership()
    await b.plugin.configResolved(await configure(b))
    await assert.rejects(
      b.plugin.buildStart.call(context, { input: [id] }),
      (error) => {
        assert(error instanceof utils.MixedBuildIntegrationError)
        assert.equal(error.requested.root, b.root.replaceAll('\\', '/'))
        assert.equal(error.active.root, a.root)
        return true
      },
    )
    assert.deepEqual(snapshot(), maps)
    assert.deepEqual(utils.getBuildOwnership(), ownership)
    await a.plugin.closeBundle()
    await a.plugin.configResolved(await configure(a))
    await a.plugin.buildStart.call(context, {
      input: [join(a.root, 'src/A.tsx')],
    })
    await a.plugin.transform.call(context, a.source, join(a.root, 'src/A.tsx'))
  }
  await b.plugin.configResolved(await configure(b))
  await b.plugin.buildStart.call(context, { input: [id] })
  const first = await b.plugin.transform.call(context, b.source, id)
  const maps = [wasm.exportClassMap(), wasm.exportFileMap(), wasm.exportSheet()]
  await b.plugin.closeBundle()
  const ended = utils.getBuildOwnership()
  const count = telemetry.extractions.filter((row) => row.id === id).length
  if (a)
    await a.plugin.transform.call(context, a.source, join(a.root, 'src/A.tsx'))
  /** @type {ReturnType<typeof utils.getBuildOwnership> | undefined} */
  let extractionLease
  telemetry.onExtract(() => {
    extractionLease = utils.getBuildOwnership()
    assert.throws(
      () =>
        utils.beginBuild(
          {},
          { integration: 'native-owned-boundary', root: b.root },
        ),
      utils.MixedBuildIntegrationError,
    )
  })
  let suspended
  let pending = Promise.resolve()
  let closing
  try {
    // When: another environment/build-watch cycle admits work on the same plugin.
    if (mode === 'pending') {
      suspended = gate('writeFile', join(b.root, 'df/compat.d.ts'))
      await b.plugin.configResolved(await configure(b))
      pending = b.plugin.buildStart.call(context, { input: [id] })
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
    assert.deepEqual(output, first)
    assert.equal(
      telemetry.extractions.filter((row) => row.id === id).length,
      count + 1,
    )
    assert(extractionLease)
    assert.equal(extractionLease.running, ended.running)
    assert(extractionLease.generation > ended.generation)
    assert.deepEqual(
      [wasm.exportClassMap(), wasm.exportFileMap(), wasm.exportSheet()],
      maps,
      'resuming build ownership must not reset frozen maps or sheet',
    )
    await b.plugin.buildStart.call(context, { input: [id] })
    assert.equal(utils.getBuildOwnership().running, ended.running)
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
