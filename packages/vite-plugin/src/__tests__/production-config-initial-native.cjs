const assert = require('node:assert/strict')
const { existsSync, readFileSync, writeFileSync } = require('node:fs')
const { join } = require('node:path')
const { put } = require('./reservation-native-fixture.cjs')
const { configFailure } = require('./config-refresh-observer.cjs')
const { DevupUI } = require('../../dist/index.cjs')
const mode = process.argv[2]
const root = process.argv[3]
assert(mode === 'missing' || mode === 'invalid')
assert(root)

async function main() {
  // Given an initially unusable inherited configuration outside the Vite root.
  const { build } = await import('vite')
  const entry = join(root, 'project/src/A.tsx')
  const attempted = join(root, 'outside/new.json')
  const css = join(root, 'project/df/css/devup-ui.css')
  const source =
    'import {Box} from "@devup-ui/react"; export const A=<Box w="$size"/>;'
  put(root, 'project/package.json', '{"name":"initial-config","type":"module"}')
  put(root, 'project/src/A.tsx', source)
  put(root, 'project/devup.json', '{"extends":["../outside/new.json"]}')
  put(root, 'outside/.keep', '')
  if (mode === 'invalid') put(root, 'outside/new.json', '{')
  const events = []
  const waiters = []
  const facts = []
  let writes = 0
  let initialError
  const next = () =>
    events.length
      ? Promise.resolve(events.shift())
      : new Promise((done) => waiters.push(done))
  const plugins = DevupUI({
    devupFile: join(root, 'project/devup.json'),
    distDir: join(root, 'project/df'),
    cssDir: join(root, 'project/df/css'),
    singleCss: true,
  })
  assert(Array.isArray(plugins))
  const start = plugins[0].buildStart
  assert.equal(typeof start, 'function')
  plugins[0].buildStart = async function (options) {
    try {
      return await start.call(this, options)
    } catch (error) {
      initialError = error
      assert(error instanceof Error)
      const fact = {
        event: 'initial-preparation-error',
        name: error.name,
        message: error.message,
        cause: error.cause instanceof Error ? error.cause.name : undefined,
      }
      facts.push(fact)
      console.info(JSON.stringify(fact))
      throw error
    }
  }
  const watcher = await build({
    root: join(root, 'project'),
    configFile: false,
    logLevel: 'silent',
    oxc: { jsx: { runtime: 'classic' } },
    plugins: [
      {
        name: 'initial-config-native-observer',
        watchChange(file, change) {
          const fact = {
            event: 'native-watch-change',
            file,
            type: change.event,
          }
          facts.push(fact)
          console.info(JSON.stringify(fact))
        },
        writeBundle() {
          writes++
        },
      },
      plugins,
    ],
    build: { lib: { entry, formats: ['es'] }, watch: {} },
  })
  assert(!Array.isArray(watcher) && 'on' in watcher)
  watcher.on('event', (event) => {
    if (event.code !== 'END' && event.code !== 'ERROR') return
    const fact = {
      event: 'initial-config-native',
      phase: event.code,
      message: event.code === 'ERROR' ? event.error.message : undefined,
    }
    facts.push(fact)
    console.info(JSON.stringify(fact))
    const waiter = waiters.shift()
    if (waiter) waiter(event)
    else events.push(event)
  })
  try {
    const failure = await next()
    assert.equal(failure.code, 'ERROR')
    assert(failure.error.message.includes(attempted))
    configFailure(
      attempted,
      mode === 'missing' ? 'ENOENT' : 'SyntaxError',
    )(initialError)
    assert.equal((await next()).code, 'END')
    assert.equal(writes, 0)
    assert.equal(existsSync(css), false)
    // When the real attempted input is created or repaired without changing its consumer.
    put(
      root,
      'outside/new.json',
      '{"theme":{"length":{"default":{"size":"37px"}}}}',
    )
    assert.equal((await next()).code, 'END')
    // Then actual native watch work emits repaired CSS, never a stale successful build.
    assert.match(readFileSync(css, 'utf8'), /37px/)
    assert.equal(readFileSync(entry, 'utf8'), source)
    assert.equal(writes, 1)
    assert(facts.some((fact) => fact.event === 'native-watch-change'))
    writeFileSync(
      join(root, 'initial-watch-evidence.json'),
      JSON.stringify({ mode, attempted, writes, facts }),
    )
  } finally {
    await watcher.close()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
