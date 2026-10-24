const assert = require('node:assert/strict')
const { readFileSync } = require('node:fs')
const { join } = require('node:path')
const { put } = require('./reservation-native-fixture.cjs')
const { DevupUI } = require('../../dist/index.cjs')
const root = process.argv[2]
assert(root)

async function main() {
  const { build } = await import('vite')
  const entry = join(root, 'project/src/A.tsx')
  const config = join(root, 'project/devup.json')
  const attempted = join(root, 'outside/new.json')
  const theme = (size) =>
    JSON.stringify({ theme: { length: { default: { size } } } })
  put(
    root,
    'project/package.json',
    '{"name":"production-config-refresh","type":"module"}',
  )
  put(
    root,
    'project/src/A.tsx',
    'import {Box} from "@devup-ui/react"; export const A=<Box w="$size"/>;',
  )
  put(root, 'project/devup.json', theme('11px'))
  put(root, 'outside/.keep', '')
  const events = []
  const waiters = []
  const next = () =>
    events.length
      ? Promise.resolve(events.shift())
      : new Promise((done) => waiters.push(done))
  const plugins = DevupUI({
    devupFile: config,
    distDir: join(root, 'project/df'),
    cssDir: join(root, 'project/df/css'),
    singleCss: true,
  })
  const watcher = await build({
    root: join(root, 'project'),
    configFile: false,
    logLevel: 'silent',
    oxc: { jsx: { runtime: 'classic' } },
    plugins: [plugins],
    build: { lib: { entry, formats: ['es'] }, watch: {} },
  })
  assert(!Array.isArray(watcher) && 'on' in watcher)
  watcher.on('event', (event) => {
    if (event.code !== 'END' && event.code !== 'ERROR') return
    console.info(
      JSON.stringify({
        event: 'production-config-native',
        phase: event.code,
        message: event.code === 'ERROR' ? event.error.message : undefined,
      }),
    )
    const waiter = waiters.shift()
    if (waiter) waiter(event)
    else events.push(event)
  })
  try {
    assert.equal((await next()).code, 'END')
    const css = join(root, 'project/df/css/devup-ui.css')
    assert.match(readFileSync(css, 'utf8'), /11px/)
    put(root, 'project/devup.json', '{"extends":["../outside/new.json"]}')
    const failure = await next()
    assert.equal(failure.code, 'ERROR')
    assert.match(failure.error.message, /new\.json:1:1:/)
    assert.match(failure.error.message, /ENOENT/)
    assert.equal((await next()).code, 'END')
    put(root, 'outside/new.json', theme('37px'))
    assert.equal((await next()).code, 'END')
    assert.match(readFileSync(css, 'utf8'), /37px/)
    assert.equal(
      readFileSync(entry, 'utf8'),
      'import {Box} from "@devup-ui/react"; export const A=<Box w="$size"/>;',
    )
    console.info(
      JSON.stringify({ event: 'production-config-recovered', attempted }),
    )
  } finally {
    await watcher.close()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
