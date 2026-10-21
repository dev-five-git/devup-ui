const assert = require('node:assert/strict')
const { rmSync, symlinkSync } = require('node:fs')
const { dirname, join, resolve } = require('node:path')
const { provenanceFixture, observe, wasm } = require('./provenance-fixture.cjs')
const { registrationBarrier } = require('./provenance-registration.cjs')
const mode = process.argv[2]
const root = process.argv[3]
assert(root)

async function main() {
  const telemetry = observe()
  const state = provenanceFixture(root, { include: ['accepted'] })
  const { plugin, write, transform } = state
  write('package.json', '{"name":"provenance-boundary","type":"module"}')
  if (mode === 'exports') {
    // Given: package-root exports above a separately named dist scope.
    const manifest = (/** @type {string} */ color) =>
      JSON.stringify({
        name: 'accepted',
        type: 'module',
        exports: './dist/' + color + '.js',
      })
    write('workspace/accepted/package.json', manifest('red'))
    write(
      'workspace/accepted/dist/package.json',
      '{"name":"named-dist","type":"module"}',
    )
    write('workspace/accepted/dist/red.js', "export const tone='red'")
    write('workspace/accepted/dist/blue.js', "export const tone='blue'")
    write('node_modules/.keep', '')
    symlinkSync(
      join(root, 'workspace/accepted'),
      join(root, 'node_modules/accepted'),
      'junction',
    )
    const source =
      "import {Box} from '@devup-ui/react'; import {tone} from 'accepted'; import '../workspace/accepted/dist/red.js'; import '../workspace/accepted/dist/blue.js'; export const A=()=> <Box color={tone}/>;"
    write('src/A.tsx', source)
    try {
      await plugin.configResolved(state.config)
      await transform(source)
      await transform(source)
      const rows = telemetry.reservations.at(-1)?.rows
      const count = telemetry.extractions.length
      // When: real package exports change with identical inventory and consumer.
      write('workspace/accepted/package.json', manifest('blue'))
      await plugin.watchChange(join(root, 'workspace/accepted/package.json'))
      await transform(source)
      // Then: actual output changes, not just a revision counter.
      assert.equal(telemetry.extractions.length, count + 1)
      assert.deepEqual(telemetry.reservations.at(-1)?.rows, rows)
      assert.match(wasm.getCss(null, false), /color:blue/)
    } finally {
      await plugin.closeBundle()
    }
    return
  }
  assert.equal(mode, 'outside-native')
  // Given: source physically outside Vite's root, watched by actual provenance.
  write('project/package.json', '{"name":"provenance-host","type":"module"}')
  write('node_modules/.keep', '')
  symlinkSync(
    dirname(
      require.resolve('react/package.json', {
        paths: [join(__dirname, '../../../react')],
      }),
    ),
    join(root, 'node_modules/react'),
    'junction',
  )
  const { createServer } = await import('vite')
  /** @type {PromiseWithResolvers<void>} */
  const ready = Promise.withResolvers()
  /** @type {ReturnType<typeof registrationBarrier> | undefined} */
  let membership
  /** @type {{type:'create'|'delete',done:PromiseWithResolvers<void>,pending:Set<string>,timestamp?:number}|undefined} */
  let semanticChange
  const server = await createServer({
    root: join(root, 'project'),
    configFile: false,
    logLevel: 'silent',
    build: { rolldownOptions: { input: state.id } },
    server: { port: 0, host: '127.0.0.1', fs: { allow: [root] } },
    plugins: [
      {
        name: 'provenance-native-registration-observer',
        enforce: 'pre',
        configureServer(server) {
          membership = registrationBarrier(server.watcher, join(root, 'src'))
          server.watcher.once('ready', () => ready.resolve())
        },
        hotUpdate: {
          order: 'post',
          handler({ file, type, timestamp }) {
            const change = semanticChange
            if (
              !change ||
              resolve(file) !== resolve(root, 'src/new-directory/B.tsx') ||
              type !== change.type
            )
              return
            change.timestamp ??= timestamp
            assert.equal(timestamp, change.timestamp)
            change.pending.delete(this.environment.name)
            console.info(
              'provenance-native semantic ' +
                JSON.stringify({
                  type,
                  timestamp,
                  pending: [...change.pending],
                }),
            )
            if (!change.pending.size) change.done.resolve()
          },
        },
      },
      state.plugins,
    ],
  })
  /** @param {string} event @param {string} file */
  function delivered(event, file) {
    return new Promise((resolveEvent) => {
      const listener = (
        /** @type {string} */ actual,
        /** @type {string} */ path,
      ) => {
        if (actual !== event || resolve(path) !== resolve(file)) return
        server.watcher.off('all', listener)
        resolveEvent(undefined)
      }
      server.watcher.on('all', listener)
    })
  }
  try {
    await server.listen()
    console.info('provenance-native waiting ready')
    await ready.promise
    const environment = server.environments.client
    assert(environment && membership)
    const loaded = await environment.transformRequest('/@fs/' + state.id)
    assert(loaded)
    const input = state.source
    const extract = () => environment.pluginContainer.transform(input, state.id)
    await extract()
    assert(
      telemetry.extractions.some(
        (item) => item.id === state.id && item.input === input,
      ),
    )
    // Global ready and actual A loading do not release this separate signal.
    await membership.ready
    membership.assertReady()
    console.info(
      'provenance-native registration ' + JSON.stringify(membership.records),
    )
    await extract()
    console.info('provenance-native ready')
    const dir = join(root, 'src/new-directory')
    const file = join(dir, 'B.tsx')
    // When: actual native outside-root directory/file create, delete, recreate.
    for (const operation of ['create', 'delete', 'recreate']) {
      const directory = delivered(
        operation === 'delete' ? 'unlinkDir' : 'addDir',
        dir,
      )
      const member = delivered(operation === 'delete' ? 'unlink' : 'add', file)
      /** @type {PromiseWithResolvers<void>} */
      const processed = Promise.withResolvers()
      semanticChange = {
        type: operation === 'delete' ? 'delete' : 'create',
        done: processed,
        pending: new Set(Object.keys(server.environments)),
      }
      const count = telemetry.extractions.length
      membership.assertReady()
      membership.beforeMutation(operation)
      if (operation === 'delete') rmSync(dir, { recursive: true })
      else write('src/new-directory/B.tsx', state.source.replace('red', 'blue'))
      console.info('provenance-native waiting ' + operation)
      await Promise.all([directory, member, processed.promise])
      console.info('provenance-native delivered ' + operation)
      await extract()
      // Then: membership dirties real preparation and rejects the old flight.
      assert.equal(telemetry.extractions.length, count + 1)
      await extract()
      assert.equal(telemetry.extractions.length, count + 1)
    }
  } finally {
    try {
      await server.close()
    } finally {
      membership?.restore()
    }
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
