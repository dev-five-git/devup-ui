const assert = require('node:assert/strict')
const { join } = require('node:path')
const { provenanceFixture, observe, wasm } = require('./provenance-fixture.cjs')

const mode = process.argv[2]
const hook = process.argv[3]
const root = process.argv[4]
assert(root)
const theme = (/** @type {readonly (string | null)[]} */ sizes) =>
  JSON.stringify({ theme: { length: { default: { size: sizes } } } })

async function main() {
  const telemetry = observe()
  const state = provenanceFixture(root)
  const { plugin, write, transform } = state
  let changed = 'devup.json'
  let source =
    "import {Box} from '@devup-ui/react'; export const A=()=> <Box w='$size'/>;"
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
          ? '@utility metadata-tone { color: red; }'
          : '@import "./imported.css";',
      )
      changed = mode === 'tailwind' ? 'selected.css' : 'imported.css'
      write(changed, '@utility metadata-tone { color: red; }')
      source =
        "import {Box} from '@devup-ui/react'; export const A=()=> <Box className='metadata-tone'/>;"
      break
    default:
      throw new TypeError('Unknown cache-invalidation metadata mode: ' + mode)
  }
  write('src/A.tsx', source)
  try {
    await plugin.configResolved(state.config)
    const before = await transform(source)
    await transform(source)
    const count = telemetry.extractions.length
    assert.equal(
      count,
      1,
      'normal configured fixture must be eligible before metadata edit',
    )
    write(
      changed,
      mode === 'root' || mode === 'inherited'
        ? theme(['11px', null, '29px'])
        : '@utility metadata-tone { color: teal; }',
    )
    const file = join(root, changed)
    const modules = new Map([[state.id, { id: state.id }]])
    /** @type {unknown[][]} */
    const invalidated = []
    const graph = {
      idToModuleMap: modules,
      getModuleById: (/** @type {string} */ id) => modules.get(id),
      invalidateModule(/** @type {unknown[]} */ ...args) {
        invalidated.push(args)
      },
    }
    const event = { file, modules: [...modules.values()], timestamp: 1401 }
    switch (hook) {
      case 'watchChange':
        await plugin.watchChange(file)
        break
      case 'hotUpdate':
        await plugin.hotUpdate.call(
          {
            environment: {
              name: 'client',
              config: { consumer: 'client' },
              moduleGraph: graph,
              hot: { send() {} },
            },
          },
          event,
        )
        assert.equal(invalidated.length, 1)
        break
      case 'handleHotUpdate':
        await plugin.handleHotUpdate({
          ...event,
          server: { moduleGraph: graph, ws: { send() {} } },
        })
        assert.equal(invalidated.length, 1)
        break
      default:
        throw new TypeError('Unknown cache-invalidation hook: ' + hook)
    }
    const after = await transform(source)
    assert.equal(
      telemetry.extractions.length,
      count + 1,
      'every metadata hook must invalidate the cached extraction',
    )
    const css = wasm.getCss(null, false)
    switch (mode) {
      case 'root':
      case 'inherited':
        assert.notDeepEqual(after, before)
        assert.match(css, /29px/)
        assert.match(css, /@media/)
        break
      case 'tailwind':
      case 'tailwind-import':
        assert.match(css, /color:teal/)
        break
      default:
        throw new TypeError('Unknown cache-invalidation metadata mode: ' + mode)
    }
    await transform(source)
    assert.equal(
      telemetry.extractions.length,
      count + 1,
      'fresh metadata extraction must then be reusable',
    )
  } finally {
    await plugin.closeBundle()
  }
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
