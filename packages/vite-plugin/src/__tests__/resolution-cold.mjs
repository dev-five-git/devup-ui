import { readdir, readFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const require = createRequire(import.meta.url)
const { build } = await import(pathToFileURL(require.resolve('vite')).href)
const { DevupUI } = await import('../../dist/index.mjs')
const root = process.argv[2]
await build({
  root,
  configFile: false,
  logLevel: 'silent',
  plugins: [DevupUI()],
  build: {
    outDir: 'cold',
    lib: {
      entry: join(root, 'src/main.js'),
      fileName: 'main',
      formats: ['es'],
    },
    minify: false,
  },
})
const files = await readdir(join(root, 'cold'))
const css = (
  await Promise.all(
    files
      .filter((name) => name.endsWith('.css'))
      .map((name) => readFile(join(root, 'cold', name), 'utf-8')),
  )
).join('\n')
if (!/background:\s*blue/.test(css) || /background:\s*red/.test(css))
  throw new Error(`Invalid cold CSS: ${css}`)
console.info(css)
