const assert = require('node:assert/strict')
const { join } = require('node:path')
const {
  provenanceFixture,
  provenanceParserGate,
  provenanceWriteGate,
  observe,
  wasm,
} = require('./provenance-fixture.cjs')
const mode = process.argv[2]
const root = process.argv[3]
assert(root)

async function main() {
  const telemetry = observe()
  const reserve = wasm.reserveClassSlots
  /** @type {(() => void) | undefined} */
  let overtake
  wasm.reserveClassSlots = (...args) => {
    reserve(...args)
    if (overtake) {
      queueMicrotask(overtake)
      overtake = undefined
    }
  }
  // Given: actual S1 configuration and a real parser suspended inside preparation.
  const parser = provenanceParserGate()
  const state = provenanceFixture(root)
  const { plugin, transform } = state
  /** @type {import('vite').ViteDevServer[]} */
  const servers = []
  try {
    if (mode === 'pending-replacement') {
      state.write(
        'devup.json',
        '{"theme":{"colors":{"default":{"primary":"red"}}}}',
      )
      const { createServer } = await import('vite')
      const first = await createServer({
        root,
        configFile: false,
        server: { watch: null },
        logLevel: 'silent',
      })
      const second = await createServer({
        root,
        configFile: false,
        server: { watch: null },
        logLevel: 'silent',
      })
      servers.push(first, second)
      const write = provenanceWriteGate(join(root, 'df/theme.d.ts'))
      const configured = plugin.configResolved(state.config)
      try {
        await write.entered
        plugin.configureServer(first)
        plugin.configureServer(second)
        write.release()
        await configured
        await transform()
        await transform()
        assert.equal(
          telemetry.extractions.length,
          2,
          'old pending completion cannot ready a replacement session',
        )
      } finally {
        write.restore()
        await configured
      }
      return
    }
    await plugin.configResolved(state.config)
    if (mode === 'physical' || mode === 'physical-query') {
      state.write(
        'index.html',
        '<script type="module" src="/src/A.tsx"></script>',
      )
      const a =
        "import {globalCss} from '@devup-ui/react'; globalCss({body:{color:'red'}});"
      const b = a.replace('red', 'blue')
      const old = transform(a)
      await parser.entered
      await plugin.configResolved(state.config)
      /** @type {Promise<unknown> | undefined} */
      let fresh
      overtake = () => {
        fresh = transform(
          b,
          state.id + (mode === 'physical-query' ? '?current' : ''),
        )
      }
      // When: the prepared fast path overtakes the obsolete pending.finally waiter.
      parser.release()
      await old
      await fresh
      assert.deepEqual(
        telemetry.extractions.map((row) => row.input),
        [b, a],
        'real extraction order must prove overtaking',
      )
      assert.match(wasm.getCss(null, false), /body\{color:red\}/)
      await transform(
        b,
        state.id + (mode === 'physical-query' ? '?current' : ''),
      )
      // Then: B restores actual global CSS rather than returning a stale cached flight.
      assert.equal(
        telemetry.extractions.length,
        3,
        'obsolete extraction must invalidate current physical observation',
      )
      assert.match(wasm.getCss(null, false), /body\{color:blue\}/)
      await transform(
        b,
        state.id + (mode === 'physical-query' ? '?current' : ''),
      )
      assert.equal(telemetry.extractions.length, 3)
    } else {
      const { createServer } = await import('vite')
      const first = await createServer({
        root,
        configFile: false,
        server: { watch: null },
        logLevel: 'silent',
      })
      servers.push(first)
      plugin.configureServer(first)
      if (mode === 'fresh') await plugin.configResolved(state.config)
      const second = await createServer({
        root,
        configFile: false,
        server: { watch: null },
        logLevel: 'silent',
      })
      servers.push(second)
      // When: attach a replacement after fresh config, or without one as a control.
      plugin.configureServer(second)
      await transform()
      await transform()
      // Then: only the successfully configured fresh session is eligible.
      assert.equal(
        telemetry.extractions.length,
        mode === 'fresh' ? 1 : 2,
        'fresh config session must survive old watcher teardown',
      )
    }
  } finally {
    wasm.reserveClassSlots = reserve
    parser.restore()
    await plugin.closeBundle()
    for (const server of servers) await server.close()
  }
}
let settled = false
process.once('beforeExit', () => {
  assert(settled, 'real lifetime scenario must settle before process exit')
})
main().then(
  () => {
    settled = true
  },
  (error) => {
    settled = true
    console.error(error)
    process.exitCode = 1
  },
)
