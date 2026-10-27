const assert = require('node:assert/strict')
const { rmSync, symlinkSync } = require('node:fs')
const { dirname, join, resolve } = require('node:path')
const { provenanceFixture, observe, wasm } = require('./provenance-fixture.cjs')
const { registrationBarrier } = require('./provenance-registration.cjs')
const {
  observeRefreshFailure,
  configFailure,
} = require('./config-refresh-observer.cjs')

const root = process.argv[2]
assert(root)
const theme = (/** @type {readonly (string | null)[]} */ size) =>
  JSON.stringify({ theme: { length: { default: { size } } } })

async function main() {
  const telemetry = observe()
  const state = provenanceFixture(root, {
    devupFile: join(root, 'project/devup.json'),
  })
  const source =
    "import {Box} from '@devup-ui/react'; export const A=()=> <Box w='$size'/>;"
  state.write(
    'project/package.json',
    '{"name":"cache-invalidation-outside-host","type":"module"}',
  )
  state.write('project/devup.json', '{"extends":["../src/base.json"]}')
  state.write('src/base.json', theme(['11px']))
  state.write('src/A.tsx', source)
  state.write('node_modules/.keep', '')
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
  /** @type {ReturnType<typeof registrationBarrier> | undefined} */
  let membership
  /** @type {{type:string,done:PromiseWithResolvers<void>,environments:Set<string>,failures:{environment:string,phase:string,error:unknown}[],timestamp?:number} | undefined} */
  let update
  const file = join(root, 'src/base.json')
  const observed = (
    /** @type {import('vite').HotUpdateOptions} */ context,
    /** @type {string} */ environment,
  ) => {
    if (
      !update ||
      resolve(context.file) !== resolve(file) ||
      context.type !== update.type
    )
      return
    update.timestamp ??= context.timestamp
    assert.equal(context.timestamp, update.timestamp)
    update.environments.delete(environment)
    if (!update.environments.size) update.done.resolve()
  }
  observeRefreshFailure(state.plugins[0], (fact, environment, error) => {
    if (fact.phase === 'hotUpdate') {
      if (update && resolve(fact.context.file) === resolve(file))
        update.failures.push({ environment, phase: fact.phase, error })
      observed(fact.context, environment)
      return
    }
    if (
      !update ||
      resolve(fact.file) !== resolve(file) ||
      fact.type !== update.type
    )
      return
    update.failures.push({ environment, phase: fact.phase, error })
    console.info(
      JSON.stringify({ event: 'config-refresh-failure', ...fact, environment }),
    )
    update.environments.delete(environment)
    if (!update.environments.size) update.done.resolve()
  })
  const server = await createServer({
    root: join(root, 'project'),
    configFile: false,
    logLevel: 'silent',
    build: { rolldownOptions: { input: state.id } },
    server: { port: 0, host: '127.0.0.1', fs: { allow: [root] } },
    plugins: [
      {
        name: 'cache-invalidation-outside-theme-observer',
        enforce: 'pre',
        configureServer(server) {
          membership = registrationBarrier(server.watcher, join(root, 'src'))
        },
        hotUpdate: {
          order: 'post',
          handler(context) {
            observed(context, this.environment.name)
          },
        },
      },
      state.plugins,
    ],
  })
  const delivered = (/** @type {string} */ expected) =>
    new Promise((done) => {
      const listener = (
        /** @type {string} */ event,
        /** @type {string} */ actual,
      ) => {
        if (event !== expected || resolve(actual) !== resolve(file)) return
        server.watcher.off('all', listener)
        done(undefined)
      }
      server.watcher.on('all', listener)
    })
  try {
    await server.listen()
    const client = server.environments.client
    assert(client && membership)
    assert(await client.transformRequest('/@fs/' + state.id))
    await membership.ready
    membership.assertReady()
    const extract = () => client.pluginContainer.transform(source, state.id)
    await extract()
    await extract()
    for (const action of ['edit', 'delete', 'recreate']) {
      const native = delivered(
        action === 'edit' ? 'change' : action === 'delete' ? 'unlink' : 'add',
      )
      /** @type {PromiseWithResolvers<void>} */
      const done = Promise.withResolvers()
      update = {
        type:
          action === 'edit'
            ? 'update'
            : action === 'delete'
              ? 'delete'
              : 'create',
        done,
        environments: new Set(
          action === 'delete'
            ? Object.values(server.environments)
                .filter(
                  (environment) =>
                    environment.name === 'client' ||
                    server.config.server.perEnvironmentWatchChangeDuringDev ||
                    state.plugins[0].perEnvironmentWatchChangeDuringDev,
                )
                .map((environment) => environment.name)
            : Object.keys(server.environments),
        ),
        failures: [],
      }
      const count = telemetry.extractions.length
      membership.assertReady()
      membership.beforeMutation(action)
      if (action === 'delete') rmSync(file)
      else
        state.write(
          'src/base.json',
          theme(
            action === 'edit' ? ['11px', null, '29px'] : ['17px', null, '37px'],
          ),
        )
      await Promise.all([native, done.promise])
      if (action === 'delete') {
        assert.deepEqual(
          update.failures.map(({ environment }) => environment).sort(),
          Object.values(server.environments)
            .filter(
              (environment) =>
                environment.name === 'client' ||
                server.config.server.perEnvironmentWatchChangeDuringDev ||
                state.plugins[0].perEnvironmentWatchChangeDuringDev,
            )
            .map((environment) => environment.name)
            .sort(),
        )
        assert.equal(
          update.timestamp,
          undefined,
          'watchChange rejects before native hotUpdate begins',
        )
        for (const { phase, error } of update.failures) {
          assert.equal(phase, 'watchChange')
          configFailure(file)(error)
        }
        await assert.rejects(extract(), configFailure(file))
        await assert.rejects(extract(), configFailure(file))
        assert.equal(telemetry.extractions.length, count)
        continue
      }
      assert.deepEqual(update.failures, [])
      await extract()
      assert.equal(
        telemetry.extractions.length,
        count + 1,
        'outside inherited native semantics must invalidate cached input',
      )
      const css = wasm.getCss(null, false)
      switch (action) {
        case 'edit':
          assert.match(css, /29px/)
          assert.match(css, /@media/)
          break
        case 'recreate':
          assert.match(css, /37px/)
          assert.match(css, /@media/)
          break
        default:
          throw new TypeError('Unknown outside theme action: ' + action)
      }
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
