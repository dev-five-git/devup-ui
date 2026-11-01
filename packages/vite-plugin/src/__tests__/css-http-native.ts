import assert from 'node:assert/strict'
import { mkdirSync, realpathSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { createServer } from 'vite'

import { DevupUI } from '../plugin'

// This helper is bundled into an owned temp directory, then run only by Node.
const root = join(
  realpathSync(dirname(fileURLToPath(import.meta.url))),
  'project',
)
const accept = process.argv[2]
assert(accept === 'text/css' || accept === '*/*')
mkdirSync(join(root, 'src'), { recursive: true })
writeFileSync(join(root, 'devup.json'), '{}')
writeFileSync(join(root, 'package.json'), '{"type":"module"}')
writeFileSync(
  join(root, 'index.html'),
  '<script type="module" src="/src/main.js"></script>',
)
writeFileSync(
  join(root, 'src/main.js'),
  "import { globalCss } from '@devup-ui/react'; globalCss({ body: { outlineStyle: 'dashed' } });",
)
process.chdir(root)

// Given: actual extraction and Vite with its default strict filesystem policy.
const server = await createServer({
  configFile: false,
  root,
  cacheDir: join(root, '.vite'),
  logLevel: 'silent',
  plugins: [
    DevupUI({
      singleCss: true,
      distDir: join(root, 'df'),
      cssDir: join(root, 'df/css'),
      devupFile: join(root, 'devup.json'),
    }),
  ],
  server: { host: '127.0.0.1', port: 0, hmr: false },
  optimizeDeps: { noDiscovery: true, include: [] },
})
try {
  await server.listen()
  const address = server.httpServer?.address()
  assert(address && typeof address === 'object')
  const base = `http://127.0.0.1:${address.port}`
  const entry = await fetch(`${base}/src/main.js`, {
    signal: AbortSignal.timeout(10000),
  })
  const transformed = await entry.text()
  assert.equal(entry.status, 200, transformed)
  const imports = [
    ...transformed.matchAll(
      /\bimport\s*["']([^"'\n]+\.css(?:\?[^"'\n]*)?)["']/g,
    ),
  ]
  assert.equal(imports.length, 1, transformed)
  const stylesheet = imports[0]?.[1]
  assert(stylesheet, transformed)

  // When: request the emitted URL unchanged, without forcing ?direct.
  const response = await fetch(new URL(stylesheet, base), {
    headers: { Accept: accept },
    signal: AbortSignal.timeout(10000),
  })
  const body = await response.text()

  // Then: useful CSS/JS delivery must never opt into immutable caching.
  assert.equal(response.status, 200, body)
  assert.match(
    response.headers.get('content-type') ?? '',
    accept === 'text/css'
      ? /^text\/css\b/i
      : /^(?:text|application)\/javascript\b/i,
  )
  assert.match(body, /body\s*\{[^}]*outline-style:\s*dashed/)
  assert.doesNotMatch(
    response.headers.get('cache-control') ?? '',
    /\bimmutable\b/i,
  )
} finally {
  await server.close()
}
