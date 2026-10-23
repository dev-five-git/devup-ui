const assert = require('node:assert/strict')
const { join } = require('node:path')
const {
  fixture,
  observe,
  wasm,
} = require('./serve-coherence-cache-fixture.cjs')
const mode = process.argv[2]
const root = process.argv[3]
assert(root)
const telemetry = observe()

async function main() {
  // Given: an exclusively owned, configured serve plugin using the real facade.
  const state = fixture(root)
  const { plugin, source, id, transform } = state
  try {
    await plugin.configResolved({
      root,
      command: mode === 'build' ? 'build' : 'serve',
    })
    const first = await transform()
    const count = telemetry.extractions.length
    // When: the exact full transform id and whole input repeat without changes.
    const repeated = await transform()
    // Then: no second real extraction, while the real sheet remains available.
    assert.equal(
      telemetry.extractions.length,
      mode === 'build' ? count + 1 : count,
      'exact input reuse must be serve-only',
    )
    assert(repeated)
    assert.match(wasm.getCss(null, false), /color:red/)
    switch (mode) {
      case 'basic':
      case 'build':
        break
      case 'input': {
        const blue = source.replace("'red'", "'blue'")
        await transform(blue, id + '?alternate')
        await transform()
        assert.equal(telemetry.extractions.length, count + 2)
        assert.match(wasm.getCss(null, false), /color:red/)
        break
      }
      case 'query': {
        await transform(source, id + '?alternate')
        assert.equal(telemetry.extractions.length, count + 1)
        await transform(source, id + '?alternate')
        assert.equal(telemetry.extractions.length, count + 1)
        break
      }
      case 'sheet': {
        const blue = source.replace("'red'", "'blue'")
        await transform(blue, id.replace('A.tsx', 'B.tsx'))
        await transform()
        assert.equal(
          telemetry.extractions.length,
          count + 1,
          'unrelated first observation must not evict A',
        )
        const css = plugin.load(join(root, 'df/css/devup-ui.css'))
        assert(css)
        assert.match(css, /color:red/)
        assert.match(css, /color:blue/)
        break
      }
      case 'membership': {
        const classes = JSON.parse(wasm.exportClassMap())
        const files = JSON.parse(wasm.exportFileMap())
        state.write('src/B.tsx', source.replace("'red'", "'blue'"))
        await plugin.watchChange(join(root, 'src/B.tsx'))
        const after = await transform()
        assert.deepEqual(after, first)
        assert.equal(telemetry.extractions.length, count + 1)
        const nextClasses = JSON.parse(wasm.exportClassMap())
        const nextFiles = JSON.parse(wasm.exportFileMap())
        assert.partialDeepStrictEqual(nextClasses, classes)
        assert.partialDeepStrictEqual(nextFiles, files)
        break
      }
      default:
        throw new TypeError('Unknown cache mode: ' + mode)
    }
  } finally {
    await plugin.closeBundle()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
