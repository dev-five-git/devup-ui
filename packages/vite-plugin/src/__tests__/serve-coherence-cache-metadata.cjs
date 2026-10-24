const assert = require('node:assert/strict')
const { rmSync } = require('node:fs')
const { join } = require('node:path')
const { configFailure } = require('./config-refresh-observer.cjs')
const {
  fixture,
  observe,
  wasm,
} = require('./serve-coherence-cache-fixture.cjs')
const mode = process.argv[2]
const root = process.argv[3]
assert(root)
const telemetry = observe()
const theme = (/** @type {readonly (string|null)[]} */ sizes) =>
  JSON.stringify({ theme: { length: { default: { size: sizes } } } })
const alias = (/** @type {string} */ color) =>
  JSON.stringify({
    compilerOptions: {
      baseUrl: '.',
      paths: { '@tone': ['./src/' + color + '.js'] },
    },
  })
const manifest = (/** @type {string} */ color) =>
  JSON.stringify({
    name: '@cache/tone',
    type: 'module',
    exports: './' + color + '.js',
  })

async function main() {
  // Given: unchanged consumer bytes and both resolution candidates in inventory.
  const state = fixture(root, { include: ['@cache/tone'] })
  const { write, plugin, transform } = state
  let source =
    "import {Box} from '@devup-ui/react'; export const A=()=> <Box w='$size'/>;"
  let changed = 'devup.json'
  switch (mode) {
    case 'root':
      write('devup.json', theme(['11px']))
      break
    case 'inherited':
      write('devup.json', '{"extends":["./config/base.json"]}')
      changed = 'config/base.json'
      write(changed, theme(['11px']))
      break
    case 'tailwind':
    case 'tailwind-import':
      write(
        'devup.json',
        JSON.stringify({ tailwind: { css: join(root, 'selected.css') } }),
      )
      write(
        'selected.css',
        mode === 'tailwind'
          ? '@utility cache-tone { color: red; }'
          : '@import "./imported.css";',
      )
      changed = mode === 'tailwind' ? 'selected.css' : 'imported.css'
      write(changed, '@utility cache-tone { color: red; }')
      source =
        "import {Box} from '@devup-ui/react'; export const A=()=> <Box className='cache-tone'/>;"
      break
    case 'alias':
    case 'exports':
      source =
        "import {Box} from '@devup-ui/react'; import {tone} from '" +
        (mode === 'alias' ? '@tone' : '@cache/tone') +
        "'; export const A=()=> <Box color={tone}/>;"
      source +=
        "import './red.js'; import './blue.js'; import '../node_modules/@cache/tone/red.js'; import '../node_modules/@cache/tone/blue.js';"
      write('src/red.js', "export const tone='red'")
      write('src/blue.js', "export const tone='blue'")
      write('node_modules/@cache/tone/red.js', "export const tone='red'")
      write('node_modules/@cache/tone/blue.js', "export const tone='blue'")
      write(
        'src/inventory.ts',
        "import './red.js'; import './blue.js'; import '@cache/tone/red.js'; import '@cache/tone/blue.js';",
      )
      changed =
        mode === 'alias'
          ? 'tsconfig.json'
          : 'node_modules/@cache/tone/package.json'
      write(changed, mode === 'alias' ? alias('red') : manifest('red'))
      break
    default:
      throw new TypeError('Unknown metadata mode ' + mode)
  }
  write('src/A.tsx', source)
  try {
    await plugin.configResolved(state.config)
    const before = await transform(source)
    await transform(source)
    const count = telemetry.extractions.length
    const rows = telemetry.reservations.at(-1)?.rows
    // When: the actual selected semantic file changes, but full id and source do not.
    switch (mode) {
      case 'root':
      case 'inherited':
        write(changed, theme(['11px', null, '29px']))
        break
      case 'tailwind':
      case 'tailwind-import':
        write(changed, '@utility cache-tone { color: teal; }')
        break
      case 'alias':
        write(changed, alias('blue'))
        break
      case 'exports':
        write(changed, manifest('blue'))
        break
      default:
        throw new TypeError('Unknown metadata mode ' + mode)
    }
    await plugin.watchChange(join(root, changed))
    const after = await transform(source)
    // Then: the real facade must run again and publish actual changed semantics.
    assert.equal(
      telemetry.extractions.length,
      count + 1,
      'metadata must invalidate eligible cached input',
    )
    const css = wasm.getCss(null, false)
    switch (mode) {
      case 'root':
      case 'inherited':
        assert.notDeepEqual(after, before)
        assert.match(css, /29px/)
        assert.match(css, /@media/)
        if (mode === 'inherited') {
          rmSync(join(root, changed))
          const missing = configFailure(join(root, changed))
          await assert.rejects(plugin.watchChange(join(root, changed)), missing)
          await assert.rejects(transform(source), missing)
          assert.equal(telemetry.extractions.length, count + 1)
          write(changed, theme(['17px', null, '37px']))
          await plugin.watchChange(join(root, changed))
          await transform(source)
          assert.equal(telemetry.extractions.length, count + 2)
          assert.match(wasm.getCss(null, false), /37px/)
        }
        break
      case 'tailwind':
      case 'tailwind-import':
        assert.match(css, /color:teal/)
        break
      case 'alias':
      case 'exports':
        assert.deepEqual(
          telemetry.reservations.at(-1)?.rows,
          rows,
          'metadata edit must keep inventory unchanged',
        )
        assert.match(css, /color:blue/)
        assert(
          telemetry.extractions
            .at(-1)
            ?.dependencies.some((file) => file.endsWith('/blue.js')),
        )
        break
      default:
        throw new TypeError('Unknown metadata mode ' + mode)
    }
  } finally {
    await plugin.closeBundle()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
