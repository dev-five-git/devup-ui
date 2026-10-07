import assert from 'node:assert/strict'
import { once } from 'node:events'
import {
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rename,
  rm,
  symlink,
  writeFile,
} from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const require = createRequire(import.meta.url)
const { createServer } = await import(
  pathToFileURL(require.resolve('vite')).href
)
const { DevupUI } = require('../../dist/index.cjs')
const fixture = await realpath(
  await mkdtemp(join(tmpdir(), 'devup-config-watch-')),
)
const root = join(fixture, 'apps/app')
const preset = join(fixture, 'packages/preset')
const manifest = join(preset, 'package.json')
let server
try {
  await mkdir(join(root, 'src'), { recursive: true })
  await mkdir(join(root, 'node_modules/@scope'), { recursive: true })
  await mkdir(preset, { recursive: true })
  await symlink(preset, join(root, 'node_modules/@scope/preset'), 'junction')
  await writeFile(join(root, 'tsconfig.json'), '{"extends":"@scope/preset"}')
  await writeFile(manifest, '{"name":"@scope/preset","exports":"./red.json"}')
  for (const color of ['red', 'blue']) {
    await writeFile(
      join(preset, `${color}.json`),
      JSON.stringify({
        compilerOptions: { paths: { palette: [`${color}.ts`] } },
      }),
    )
    await writeFile(
      join(preset, `${color}.ts`),
      `export const color='${color}'`,
    )
  }
  await writeFile(
    join(root, 'src/main.tsx'),
    "import {color} from 'palette'; import {css} from '@devup-ui/react'; export const style=css({bg:color})",
  )
  let ready
  let notify
  const update = new Promise((done) => {
    notify = done
  })
  const signal = AbortSignal.timeout(15000)
  server = await createServer({
    root,
    configFile: false,
    optimizeDeps: { noDiscovery: true },
    server: { port: 0 },
    plugins: [
      DevupUI(),
      {
        name: 'config-watch-observation',
        configureServer(current) {
          ready = once(current.watcher, 'ready', { signal })
        },
        hotUpdate({ file }) {
          if (file === manifest.replaceAll('\\', '/')) notify()
        },
        resolveId(id) {
          if (id === 'palette') return join(preset, 'red.ts')
        },
      },
    ],
  })
  await server.listen()
  const before = await server.transformRequest('/src/main.tsx')
  await ready
  const cssPath = join(root, 'df/devup-ui/devup-ui-0.css')
  assert.match(await readFile(cssPath, 'utf8'), /background:red/)
  const pid = process.pid
  // Only the selecting manifest changes; watcher transport is Vite's public API.
  const replacement = join(fixture, 'replacement.json')
  await writeFile(
    replacement,
    '{"name":"@scope/preset","exports":"./blue.json"}',
  )
  await rename(replacement, manifest)
  await Promise.race([
    update,
    new Promise((_, reject) => {
      signal.addEventListener('abort', () => reject(signal.reason), {
        once: true,
      })
    }),
  ])
  const after = await server.transformRequest('/src/main.tsx')
  assert.notEqual(after?.code, before?.code)
  const selected = after?.code.match(/style\s*=\s*["']([^"']+)["']/)?.[1]
  assert.ok(selected)
  const css = await readFile(cssPath, 'utf8')
  assert.ok(css.includes(`.${selected}{background:blue}`), css)
  assert.equal(process.pid, pid)
  console.info(
    JSON.stringify({
      runtime: process.version,
      pid,
      before: before?.code,
      after: after?.code,
      selected,
      css,
    }),
  )
} finally {
  await server?.close()
  await rm(fixture, { recursive: true, force: true })
}
