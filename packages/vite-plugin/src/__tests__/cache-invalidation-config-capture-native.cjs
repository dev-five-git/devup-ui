const assert = require('node:assert/strict')
const { join } = require('node:path')
const { provenanceFixture, observe, wasm } = require('./provenance-fixture.cjs')
const { gate } = require('./serve-coherence-async-fixture.cjs')

const root = process.argv[2]
assert(root)
const theme = (/** @type {readonly (string | null)[]} */ size) =>
  JSON.stringify({ theme: { length: { default: { size } } } })

async function main() {
  const telemetry = observe()
  const state = provenanceFixture(root)
  const source =
    "import {Box} from '@devup-ui/react'; export const A=()=> <Box w='$size'/>;"
  state.write('src/A.tsx', source)
  state.write('devup.json', theme(['11px']))
  await state.plugin.configResolved(state.config)
  const css = gate('writeFile', join(root, 'df/css/devup-ui.css'))
  const old = state.transform(source)
  const oldSettled = old.then(() => true)
  /** @type {ReturnType<typeof gate> | undefined} */
  let config
  let refreshed = Promise.resolve()
  /** @type {Promise<unknown> | undefined} */
  let newer
  try {
    await css.entered
    state.write('devup.json', theme(['11px', null, '29px']))
    config = gate('readFile', join(root, 'devup.json'))
    refreshed = state.plugin.watchChange(join(root, 'devup.json'))
    await config.entered
    await state.transform(source)
    await state.transform(source)
    assert.equal(
      telemetry.extractions.length,
      3,
      'active independent config load must bypass retention and reuse',
    )
    config.release()
    await refreshed
    assert.match(wasm.getCss(null, false), /29px/)
    assert.equal(
      await Promise.race([oldSettled, Promise.resolve(false)]),
      false,
      'new semantics must finish before the old CSS write returns',
    )
    /** @type {PromiseWithResolvers<void>} */
    const extracted = Promise.withResolvers()
    telemetry.onExtract(() => extracted.resolve())
    newer = state.transform(source)
    await extracted.promise
    assert.equal(telemetry.extractions.length, 4)
    css.release()
    const result = await newer
    await old
    assert.deepEqual(await state.transform(source), result)
    assert.equal(
      telemetry.extractions.length,
      4,
      'obsolete successful completion must not delete or replace the newer flight',
    )
    assert.match(wasm.getCss(null, false), /29px/)
    assert.match(wasm.getCss(null, false), /@media/)
  } finally {
    config?.restore()
    css.restore()
    await Promise.allSettled([old, newer, refreshed])
    await state.plugin.closeBundle()
  }
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
