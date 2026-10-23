const assert = require('node:assert/strict')
const { join } = require('node:path')
const wasm = require('@devup-ui/wasm')
const { project, gate } = require('./serve-coherence-async-fixture.cjs')
const stage = process.argv[2]
assert(stage === 'load' || stage === 'write')
/** @param {string} size */
const theme = (size) =>
  JSON.stringify({ theme: { length: { default: { size } } } })

async function main() {
  // Given: an old refresh suspended with actual old bytes or after theme application.
  const state = project()
  await state.plugin.configResolved(state.config)
  state.write('old.json', theme('11px'))
  state.write('latest.json', theme('29px'))
  state.write('old.css', '@utility serve-coherence-tone { color: red; }')
  state.write('latest.css', '@utility serve-coherence-tone { color: blue; }')
  state.write(
    'devup.json',
    JSON.stringify({
      extends: ['./old.json'],
      tailwind: { css: join(state.root, 'old.css') },
    }),
  )
  const suspended = gate(
    stage === 'load' ? 'readFile' : 'writeFile',
    join(state.root, stage === 'load' ? 'devup.json' : 'df/theme.d.ts'),
  )
  let old = Promise.resolve()
  let latest = Promise.resolve()
  let nextRead
  try {
    old = state.plugin.watchChange(join(state.root, 'devup.json'))
    await suspended.entered
    state.write(
      'devup.json',
      JSON.stringify({
        extends: ['./latest.json'],
        tailwind: { css: join(state.root, 'latest.css') },
      }),
    )
    if (stage === 'write')
      nextRead = gate('readFile', join(state.root, 'devup.json'))
    // When: the next request arrives before the old transaction can finish.
    latest = state.plugin.watchChange(join(state.root, 'devup.json'))
    // A concurrent implementation can finish the newer request first. A serial
    // implementation cannot enter its read until release; neither branch sleeps.
    if (stage === 'load' && suspended.calls > 1) await latest
    nextRead?.release()
    if (nextRead && nextRead.calls > 0) await latest
    suspended.release()
    await Promise.all([old, latest])
    // Then: newest theme and newest provenance must describe the same transaction.
    assert.match(
      wasm.getCss(null, false),
      /--size:29px/,
      'newest requested theme must win',
    )
    state.write('latest.json', theme('47px'))
    await state.plugin.watchChange(join(state.root, 'latest.json'))
    assert.match(
      wasm.getCss(null, false),
      /--size:47px/,
      'latest inherited input must still trigger refresh',
    )
    state.write('latest.json', theme('59px'))
    state.write('latest.css', '@utility serve-coherence-tone { color: teal; }')
    await state.plugin.watchChange(join(state.root, 'latest.css'))
    assert.match(
      wasm.getCss(null, false),
      /--size:59px/,
      'latest Tailwind input must still trigger refresh',
    )
    const result = await state.plugin.transform.call(
      { addWatchFile() {} },
      state.source,
      join(state.root, 'src/A.tsx'),
    )
    assert(result)
    assert.match(wasm.getCss(null, false), /59px/)
    console.info('serve-coherence-async-complete')
  } finally {
    nextRead?.restore()
    suspended.restore()
    await Promise.all([old, latest])
    await state.close()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
