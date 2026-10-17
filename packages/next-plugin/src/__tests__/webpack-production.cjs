const assert = require('node:assert/strict')
const { spawn } = require('node:child_process')
const {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} = require('node:fs')
const { createRequire } = require('node:module')
const { dirname, join, resolve } = require('node:path')

const repository = resolve(__dirname, '../../../..')
const fromPlugin = createRequire(
  join(repository, 'packages/next-plugin/package.json'),
)
const fromNext = createRequire(fromPlugin.resolve('next/package.json'))
const root = process.env.DEVUP_NEXT_FIXTURE_ROOT
assert.ok(
  root,
  'run through webpack-owned.cjs so cleanup has an external owner',
)
const fault = process.argv[2] || ''
function put(name, source) {
  const path = join(root, name)
  mkdirSync(dirname(path), { recursive: true })
  writeFileSync(path, source)
}
function link(name, target) {
  const destination = join(root, 'node_modules', name)
  mkdirSync(dirname(destination), { recursive: true })
  symlinkSync(
    realpathSync(target),
    destination,
    process.platform === 'win32' ? 'junction' : 'dir',
  )
}
function contents(directory, extension) {
  return readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const path = join(directory, entry.name)
      return entry.isDirectory()
        ? contents(path, extension)
        : entry.name.endsWith(extension)
          ? [readFileSync(path, 'utf8')]
          : []
    })
    .join('\n')
}
async function run() {
  let server
  try {
    if (fault === 'build-timeout') {
      put(
        'next.config.js',
        `require('node:child_process').spawn(process.execPath, [${JSON.stringify(join(__dirname, 'webpack-lifecycle-child.cjs'))}], { stdio: 'inherit' }); module.exports = async () => new Promise(() => {});`,
      )
    }
    const build = spawn(
      process.execPath,
      [fromPlugin.resolve('next/dist/bin/next'), 'build', '--webpack'],
      {
        cwd: root,
        stdio: 'inherit',
        env: { ...process.env, NEXT_TELEMETRY_DISABLED: '1' },
      },
    )
    console.info(`W65-BUILD-PID ${build.pid}`)
    const status = await new Promise((resolve, reject) => {
      const timeout = setTimeout(
        () => reject(new Error('Next build deadline exceeded')),
        fault === 'build-timeout' ? 5000 : 60000,
      )
      build.once('error', (error) => {
        clearTimeout(timeout)
        reject(error)
      })
      build.once('exit', (code) => {
        clearTimeout(timeout)
        resolve(code)
      })
    })
    assert.equal(status, 0, 'actual next build --webpack failed')
    assert.doesNotMatch(
      contents(join(root, '.next/static/chunks'), '.js'),
      /Cannot run on the runtime/,
    )
    server = spawn(
      process.execPath,
      [
        fromPlugin.resolve('next/dist/bin/next'),
        'start',
        '--hostname',
        '127.0.0.1',
        '--port',
        '0',
      ],
      {
        cwd: root,
        env: { ...process.env, NEXT_TELEMETRY_DISABLED: '1' },
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    )
    console.info(`W65-SERVER-PID ${server.pid}`)
    const base = await new Promise((resolve, reject) => {
      let output = ''
      const timeout = setTimeout(
        () => reject(new Error('Next production server did not become ready')),
        15000,
      )
      server.once('error', (error) => {
        clearTimeout(timeout)
        reject(error)
      })
      server.once('exit', (code, signal) => {
        clearTimeout(timeout)
        reject(
          new Error(
            `Next production server exited before readiness: ${code}/${signal}`,
          ),
        )
      })
      server.stderr.on('data', (chunk) => process.stderr.write(chunk))
      server.stdout.on('data', (chunk) => {
        process.stdout.write(chunk)
        output += chunk.toString()
        const address = output.match(/http:\/\/(?:127\.0\.0\.1|localhost):\d+/)
        if (address && output.includes('Ready in')) {
          clearTimeout(timeout)
          resolve(address[0])
        }
      })
    })
    writeFileSync(join(root, 'ready'), String(server.pid))
    if (fault === 'post-start-failure')
      throw new Error('Injected failure after server readiness')
    if (fault === 'driver-death') process.kill(process.pid, 'SIGKILL')
    if (fault === 'post-start-timeout') await new Promise(() => {})
    const response = await fetch(base, { signal: AbortSignal.timeout(10000) })
    assert.equal(response.status, 200)
    const html = await response.text()
    assert.match(html, /id="server-proof" class="[^"]+"/)
    const edge = await fetch(`${base}/edge`, {
      signal: AbortSignal.timeout(10000),
    })
    assert.equal(edge.status, 200)
    const edgeBody = await edge.json()
    assert.match(edgeBody.edge, /^[\w -]+$/)
    console.info(
      JSON.stringify({
        ssr: 'server-proof has compiled classes',
        edge: edgeBody,
      }),
    )
    const delivery = require('./webpack-delivery-http.cjs')
    const parse = fromNext('postcss').parse
    const documents = await delivery(base, parse)
    const emitted = delivery.rules(
      contents(join(root, '.next/static/css'), '.css'),
      parse,
    )
    delivery.declarationProof(
      emitted,
      edgeBody.edge.split(/\s+/).map((name) => `.${name}`),
      { 'border-color': 'goldenrod' },
    )
    console.info(
      'W65-API-DISK ' +
        JSON.stringify({
          classes: edgeBody.edge,
          scope: 'emitted CSS only; API has no HTML stylesheet contract',
        }),
    )
    return documents
  } finally {
    if (
      server?.pid !== undefined &&
      server.exitCode === null &&
      server.signalCode === null
    ) {
      await new Promise((resolve) => {
        const timeout = setTimeout(resolve, 1000)
        server.once('exit', () => {
          clearTimeout(timeout)
          resolve()
        })
        server.kill('SIGTERM')
      })
    }
    server?.stdout.destroy()
    server?.stderr.destroy()
    // Only the external owner may remove root, after its entire tree has exited.
  }
}
async function main() {
  // Fixture shape and dependency junctions credit W64c's physical Next evidence.
  for (const name of ['next', 'react', 'react-dom'])
    link(name, dirname(fromNext.resolve(`${name}/package.json`)))
  for (const name of ['react', 'next-plugin'])
    link(`@devup-ui/${name}`, join(repository, 'packages', name))
  put(
    'package.json',
    JSON.stringify({ name: 'w65-next-webpack', private: true }),
  )
  put(
    'src/app/edge/route.js',
    `import { css } from '@devup-ui/react'; export const runtime = 'edge'; const edge = css({ borderColor: 'goldenrod' }); export function GET() { return Response.json({ edge }); }`,
  )
  require('./webpack-delivery-fixture.cjs')(put)
  let baseline
  for (const cache of fault ? ['cold'] : ['cold', 'cold-repeat', 'warm']) {
    if (cache === 'cold-repeat') {
      rmSync(join(root, '.next'), { recursive: true, force: true })
      rmSync(join(root, 'df'), { recursive: true, force: true })
    }
    console.info('W65-CACHE ' + cache)
    if (cache === 'warm') {
      const directory = join(root, '.next/cache/webpack')
      const files = readdirSync(directory, { recursive: true })
        .filter((name) => /\.pack(?:\.gz|\.br)?$/.test(name))
        .map((name) => ({ name, bytes: statSync(join(directory, name)).size }))
      assert.ok(
        files.some(({ bytes }) => bytes > 0),
        'warm run requires populated webpack cache',
      )
      console.info('W65-WARM-CACHE ' + JSON.stringify(files))
    } else
      assert.equal(
        existsSync(join(root, '.next')),
        false,
        `${cache} must start without Next output/cache`,
      )
    const actual = await run()
    if (baseline)
      assert.deepEqual(
        actual,
        baseline,
        `${cache}: HTTP CSS/class correspondence changed`,
      )
    else baseline = actual
  }
  console.info('webpack production proof passed')
}
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
