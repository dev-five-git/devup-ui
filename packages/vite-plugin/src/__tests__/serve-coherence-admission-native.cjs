const assert = require('node:assert/strict')
const { join } = require('node:path')
const wasm = require('@devup-ui/wasm')
const { project, gate } = require('./serve-coherence-async-fixture.cjs')
const mode = process.argv[2]
assert(mode === 'config' || mode === 'tailwind' || mode === 'tailwind-import')
/** @param {string} size */
const theme = (size) =>
  JSON.stringify({ theme: { length: { default: { size } } } })

async function main() {
  // Given: completed old inputs, then a newly selected input already under src.
  const state = project()
  state.write('old.json', theme('11px'))
  state.write('src/latest.json', theme('29px'))
  state.write('old.css', '@utility coherence-tone { color: red; }')
  state.write('src/tone.css', '@utility coherence-tone { color: blue; }')
  state.write(
    'src/latest.css',
    mode === 'tailwind-import'
      ? '@import "./tone.css";'
      : '@utility coherence-tone { color: blue; }',
  )
  state.write(
    'devup.json',
    JSON.stringify({
      extends: ['./old.json'],
      tailwind: { css: join(state.root, 'old.css') },
    }),
  )
  await state.plugin.configResolved(state.config)
  state.write(
    'devup.json',
    JSON.stringify({
      extends: ['./src/latest.json'],
      tailwind: { css: join(state.root, 'src/latest.css') },
    }),
  )
  const suspended = gate('writeFile', join(state.root, 'df/theme.d.ts'))
  let refresh = Promise.resolve()
  let event = Promise.resolve()
  try {
    refresh = state.plugin.watchChange(join(state.root, 'devup.json'))
    await suspended.entered
    assert.match(wasm.getCss(null, false), /--size:29px/)
    // When: the selected input event arrives BEFORE old write completion.
    state.write('src/latest.json', theme('47px'))
    const changed =
      mode === 'config'
        ? 'src/latest.json'
        : mode === 'tailwind'
          ? 'src/latest.css'
          : 'src/tone.css'
    if (mode !== 'config')
      state.write(changed, '@utility coherence-tone { color: teal; }')
    event = state.plugin.watchChange(join(state.root, changed))
    suspended.release()
    await Promise.all([refresh, event])
    // Then: event admission must reapply actual newest bytes, not only reserve.
    assert.match(
      wasm.getCss(null, false),
      /--size:47px/,
      'active discovered input event must enqueue the next refresh',
    )
    if (mode !== 'config') {
      const source =
        "import {Box} from '@devup-ui/react'; export const B=()=> <Box className='coherence-tone'/>;"
      const output = await state.plugin.transform.call(
        { addWatchFile() {} },
        source,
        join(state.root, 'src/B.tsx'),
      )
      assert(output)
      assert.match(wasm.getCss(null, false), /color:teal/)
    }
    console.info('serve-coherence-final-complete')
  } finally {
    suspended.restore()
    await Promise.all([refresh, event])
    await state.close()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
