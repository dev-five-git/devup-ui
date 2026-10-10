import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { basename, dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  bundleSourceEntries,
  nextSourceEntries,
  pluginUtilsFixtureManifest,
  webpackFixtureManifest,
} from './native-fixture-bundle.mjs'

const workspace = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const installed = createRequire(join(workspace, 'apps/landing/package.json'))
const mode = process.argv[2]
assert.ok(mode === 'per-file' || mode === 'singleCss', 'expected CSS mode')
const singleCss = mode === 'singleCss'
const evidenceDir = process.env.DEVUP_CSS_EVIDENCE_DIR
const root = mkdtempSync(
  join(evidenceDir ?? tmpdir(), `devup-next-css-${mode}-`),
)
const marker = 'native-devup-css-marker'
const links = []
const bundles = []
const write = (path, content) => {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, content)
}
const link = (name, target) => {
  const destination = join(root, 'node_modules', name)
  mkdirSync(dirname(destination), { recursive: true })
  const physical = realpathSync(target)
  symlinkSync(
    physical,
    destination,
    process.platform === 'win32' ? 'junction' : 'dir',
  )
  links.push({ name, physical })
}
let output = ''
let status = null
let signal = null
let timedOut = false
let closed = false
let timer
let html
let css = []
let diagnostics
const started = Date.now()
try {
  // Given: real installed dependencies and privately bundled current owned source.
  for (const name of readdirSync(
    join(workspace, 'apps/landing/node_modules'),
  )) {
    if (name === '.bin' || name === '@devup-ui') continue
    link(name, join(workspace, 'apps/landing/node_modules', name))
  }
  for (const [name, path] of Object.entries({
    react: 'packages/react',
    wasm: 'bindings/devup-ui-wasm',
  }))
    link(`@devup-ui/${name}`, join(workspace, path))
  write(
    'node_modules/@devup-ui/next-plugin/package.json',
    JSON.stringify({
      name: '@devup-ui/next-plugin',
      type: 'module',
      exports: './index.mjs',
    }),
  )
  write(
    'node_modules/@devup-ui/webpack-plugin/package.json',
    JSON.stringify(webpackFixtureManifest),
  )
  write(
    'node_modules/@devup-ui/plugin-utils/package.json',
    JSON.stringify(pluginUtilsFixtureManifest),
  )
  bundles.push(
    ...(await bundleSourceEntries(workspace, root, nextSourceEntries)),
  )
  write('package.json', JSON.stringify({ private: true, type: 'module' }))
  write(
    'app/layout.jsx',
    'export default function Layout({children}) {return <html><body>{children}</body></html>}',
  )
  write(
    'app/page.jsx',
    `import {Box} from '@devup-ui/react'; export default function Page(){return <Box as="section" bg="red">${marker}</Box>}`,
  )
  write(
    'next.config.mjs',
    `import {DevupUI} from '@devup-ui/next-plugin';
const contexts=new WeakMap();let nextContext=0;
const config=DevupUI({experimental:{cpus:1},webpack(config,context){
 if(!contexts.has(context.config))contexts.set(context.config,++nextContext);
 console.info('NATIVE_OWNER_CONTEXT '+JSON.stringify({config:contexts.get(context.config),dev:context.dev,isServer:context.isServer,nextRuntime:context.nextRuntime,buildId:context.buildId}));
 config.plugins.push({apply(compiler){compiler.hooks.thisCompilation.tap('NativeCssDeliveryObserver',compilation=>{compilation.hooks.finishModules.tap('NativeCssDeliveryObserver',()=>{
  const loaders=[...compilation.modules].filter(module=>module.resource?.endsWith('.css')).flatMap(module=>module.loaders??[]).map(loader=>loader.loader);
  console.info('NATIVE_OWNER_LOADERS '+JSON.stringify(loaders));
 })})}});return config;
}},{singleCss:${singleCss}});
if(typeof config.webpack!=='function'||config.experimental.webpackBuildWorker!==undefined) throw new TypeError('use native default custom-webpack OFF'); export default config;`,
  )
  const env = {
    ...process.env,
    NODE_OPTIONS: '',
    NODE_ENV: 'production',
    NEXT_TELEMETRY_DISABLED: '1',
    DEBUG: 'next:build:webpack-build',
  }
  for (const key of [
    'TURBOPACK',
    'NEXT_TURBOPACK_USE_WORKER',
    'NEXT_PRIVATE_BUILD_WORKER',
    'NEXT_RSPACK',
    'NEXT_PRIVATE_LOCAL_WEBPACK',
  ])
    delete env[key]
  // When: native Next build, no private lifecycle or asset intervention.
  const child = spawn(
    'node',
    [installed.resolve('next/dist/bin/next'), 'build', '--webpack'],
    { cwd: root, env, stdio: ['ignore', 'pipe', 'pipe'] },
  )
  child.stdout.on('data', (chunk) => {
    output += chunk.toString()
  })
  child.stderr.on('data', (chunk) => {
    output += chunk.toString()
  })
  const completed = new Promise((done, reject) => {
    child.once('error', reject)
    child.once('close', (code, reason) => {
      closed = true
      status = code
      signal = reason
      done()
    })
  })
  timer = setTimeout(() => {
    timedOut = true
    if (process.platform === 'win32' && child.pid !== undefined) {
      spawnSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], {
        encoding: 'utf8',
        timeout: 10000,
      })
    } else child.kill()
  }, 180000)
  await completed
  clearTimeout(timer)
  assert.equal(status, 0, output)
  assert.equal(timedOut, false, output)
  assert.equal(closed, true)
  html = readFileSync(join(root, '.next/server/app/index.html'), 'utf8')
  diagnostics = JSON.parse(
    readFileSync(
      join(root, '.next/diagnostics/build-diagnostics.json'),
      'utf8',
    ),
  )
  assert.equal(diagnostics.buildOptions.useBuildWorker, 'false')
  const contexts = output
    .split(/\r?\n/)
    .filter((line) => line.startsWith('NATIVE_OWNER_CONTEXT '))
    .map((line) => JSON.parse(line.slice('NATIVE_OWNER_CONTEXT '.length)))
  assert.equal(contexts.length, 3)
  assert.ok(contexts.every((context) => context.config === 1))
  assert.deepEqual(
    contexts.map((context) => context.nextRuntime ?? 'client').sort(),
    ['client', 'edge', 'nodejs'],
  )
  const loaders = output
    .split(/\r?\n/)
    .filter((line) => line.startsWith('NATIVE_OWNER_LOADERS '))
    .flatMap((line) => JSON.parse(line.slice('NATIVE_OWNER_LOADERS '.length)))
    .filter((path) => /webpack-plugin[\\/]css-loader\.cjs$/.test(path))
  assert.ok(loaders.length > 0)
  assert.ok(loaders.every((path) => path.startsWith(root)))
  // Then: the native class selector must occur in a stylesheet linked by native HTML.
  const section =
    /<section[^>]*class="([^"]+)"[^>]*>native-devup-css-marker<\/section>/.exec(
      html,
    )
  assert.ok(section, 'native transformed section missing')
  const assets = [
    ...html.matchAll(
      /<link\b[^>]*rel="stylesheet"[^>]*href="([^"?]+\.css)(?:[^"\s]*)"/g,
    ),
  ].map((match) => match[1])
  assert.ok(assets.length, 'native HTML stylesheet links missing')
  css = assets.map((asset) => ({
    asset,
    content: readFileSync(
      join(root, '.next', asset.replace(/^\/_next\//, '')),
      'utf8',
    ),
  }))
  const selectors = section[1]
    .split(/\s+/)
    .map((cls) => `.${cls}{background:red}`)
  assert.ok(
    css.some(({ content }) =>
      selectors.some((selector) => content.includes(selector)),
    ),
    'HTML-linked native class CSS missing; disk CSS is not acceptance',
  )
} finally {
  clearTimeout(timer)
  const evidence = {
    mode,
    workspace,
    root,
    status,
    signal,
    timedOut,
    closed,
    elapsedMs: Date.now() - started,
    links,
    bundles,
    html,
    css,
    diagnostics,
    output,
  }
  if (evidenceDir) {
    writeFileSync(
      join(evidenceDir, `${basename(root)}.json`),
      JSON.stringify(evidence, null, 2),
    )
    writeFileSync(join(evidenceDir, `${basename(root)}.log`), output)
  }
  console.info(JSON.stringify(evidence))
  rmSync(root, { recursive: true, force: true })
}
