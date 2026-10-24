const assert = require('node:assert/strict')
const { join } = require('node:path')
const wasm = require('@devup-ui/wasm')
/** @type {typeof import('../../../plugin-utils/src/index')} */
const utils = require('@devup-ui/plugin-utils')
const { project, gate } = require('./serve-coherence-async-fixture.cjs')
const { rebindParser } = require('./html-parser-bundle.cjs')
const stage = process.argv[2]
assert(['before-refresh', 'load', 'buildStart', 'transform'].includes(stage))

async function main() {
  // Given: A is active; B has real engine-mutating work suspended at an await.
  /** @type {PromiseWithResolvers<void>} */
  const entered = Promise.withResolvers()
  /** @type {PromiseWithResolvers<void>} */
  const release = Promise.withResolvers()
  let restoreParser = () => {}
  if (stage === 'buildStart' || stage === 'transform') {
    restoreParser = rebindParser((parsing) => async (...args) => {
      entered.resolve()
      await release.promise
      return parsing(...args)
    })
  }
  const a = project()
  await a.plugin.configResolved(a.config)
  const b = project()
  b.write('devup.json', '{"theme":{"length":{"default":{"size":"73px"}}}}')
  let suspended
  let work = Promise.resolve()
  let closing
  try {
    if (stage === 'before-refresh' || stage === 'load') {
      suspended = gate(
        stage === 'load' ? 'readFile' : 'writeFile',
        join(b.root, stage === 'load' ? 'devup.json' : 'df/compat.d.ts'),
      )
      work = b.plugin.configResolved(b.config)
      await suspended.entered
    } else {
      await b.plugin.configResolved(b.config)
      work =
        stage === 'buildStart'
          ? b.plugin.buildStart.call({ addWatchFile() {} })
          : b.plugin.transform
              .call({ addWatchFile() {} }, b.source, join(b.root, 'src/A.tsx'))
              .then(() => {})
      await entered.promise
    }
    const overlap = utils.getBuildOwnership()
    assert.equal(overlap.running, 2)
    // When: close is requested while B can still mutate the shared facade.
    closing = b.plugin.closeBundle()
    const during = utils.getBuildOwnership()
    suspended?.release()
    release.resolve()
    await work
    await closing
    // Then: no apparent exclusive A ownership before B's last mutation finishes.
    assert.equal(
      during.running,
      2,
      'pending B must retain ownership until its work drains',
    )
    assert.equal(during.generation, overlap.generation)
    assert.equal(utils.getBuildOwnership().running, 1)
    assert.equal(utils.getBuildOwnership().generation, overlap.generation + 1)
    assert.match(
      wasm.getCss(null, false),
      /--size:73px/,
      'configured B still applies to the actual shared engine',
    )
    console.info('serve-coherence-async-complete')
  } finally {
    suspended?.restore()
    release.resolve()
    await work
    await closing
    restoreParser()
    await b.close()
    await a.close()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
