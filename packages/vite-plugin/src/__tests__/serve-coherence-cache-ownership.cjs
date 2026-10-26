const assert = require('node:assert/strict')
const { join } = require('node:path')
const {
  fixture,
  observe,
  wasm,
} = require('./serve-coherence-cache-fixture.cjs')
const { getBuildOwnership } = require('@devup-ui/plugin-utils')
const root = process.argv[2]
assert(root)
const telemetry = observe()
async function main() {
  // Given: A is exclusive and has a completed cache entry.
  const a = fixture(root, {
    prefix: 'cacheA',
    shorthands: { cacheSpace: ['marginLeft'] },
  })
  const source =
    "import {Box} from '@devup-ui/react'; export const A=()=> <Box cacheSpace={3}/>;"
  a.write('src/A.tsx', source)
  await a.plugin.configResolved(a.config)
  await a.transform(source)
  const count = telemetry.extractions.length
  const b = fixture(join(root, 'B'), {
    prefix: 'cacheB',
    shorthands: { cacheSpace: ['paddingRight'] },
  })
  try {
    // When: B actually configures distinct engine options while both owners live.
    await b.plugin.configResolved(b.config)
    assert.equal(getBuildOwnership().running, 2)
    await a.transform(source)
    await a.transform(source)
    assert.equal(
      telemetry.extractions.length,
      count + 2,
      'overlap must bypass both lookup and publication',
    )
    assert.match(telemetry.extractions.at(-1)?.code ?? '', /cacheB/)
    assert.match(wasm.getCss(null, false), /padding-right:12px/)
    await b.plugin.closeBundle()
    assert.equal(getBuildOwnership().running, 1)
    await a.transform(source)
    await a.transform(source)
    // Then: return to exclusive forces one fresh extraction, using actual shared B options.
    assert.equal(telemetry.extractions.length, count + 3)
    assert.match(telemetry.extractions.at(-1)?.code ?? '', /cacheB/)
  } finally {
    await b.plugin.closeBundle()
    await a.plugin.closeBundle()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
