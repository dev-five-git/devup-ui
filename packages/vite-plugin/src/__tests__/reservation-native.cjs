const assert = require('node:assert/strict')
const { writeFileSync } = require('node:fs')
const { basename, resolve } = require('node:path')
const wasm = require('@devup-ui/wasm')
const { observe } = require('./reservation-native-observer.cjs')

const { text } = require('./reservation-native-receipt.cjs')
const args = process.argv.slice(2)
const root = text(args[0]),
  order = text(args[1]),
  mode = text(args[2]),
  single = text(args[3]),
  receipt = text(args[4])
process.chdir(root)
const telemetry = observe()
const { events, reservations, extractions } = telemetry
const gate = Promise.withResolvers()
telemetry.onExtract((id) => {
  if (basename(id).endsWith(`${order[0]}.js`)) gate.resolve(undefined)
})
/** @type {{DevupUI:(options: {distDir:string,cssDir:string,devupFile:string,singleCss:boolean})=>import('vite').PluginOption}} */
const { DevupUI } = require('../../dist/' + 'index.cjs')

async function main() {
  const { build, createServer } = await import('vite')
  /** @type {import('vite').Plugin} */
  const observer = {
    name: 'native-slot-arrival-gate',
    enforce: 'pre',
    async transform(_code, id) {
      if (basename(id).endsWith(`${order[1]}.js`)) await gate.promise
    },
  }
  /** @type {import('vite').InlineConfig} */
  const config = {
    configFile: false,
    root,
    logLevel: 'error',
    plugins: [
      observer,
      DevupUI({
        distDir: resolve(root, 'df'),
        cssDir: resolve(root, 'df/devup-ui'),
        devupFile: resolve(root, 'devup.json'),
        singleCss: single === 'single',
      }),
    ],
    build: {
      minify: false,
      sourcemap: true,
      outDir: 'out',
      rolldownOptions: {
        output: {
          entryFileNames: '[name].js',
          chunkFileNames: '[name].js',
          assetFileNames: '[name][extname]',
        },
      },
    },
    server: { host: '127.0.0.1', port: 0, fs: { strict: false } },
    optimizeDeps: { noDiscovery: true, include: [] },
  }
  /** @type {Record<string,string> | undefined} */
  let assets, http
  if (mode === 'build') {
    const result = await build(config)
    assert(!Array.isArray(result) && 'output' in result)
    assets = Object.fromEntries(
      result.output.map((item) => [
        item.fileName,
        item.type === 'chunk' ? item.code : String(item.source),
      ]),
    )
  } else {
    const server = await createServer(config)
    try {
      await server.listen()
      const address = server.httpServer?.address()
      assert(address && typeof address === 'object')
      const base = `http://127.0.0.1:${address.port}`
      const responses = await Promise.all(
        ['/src/A.js', '/src/B.js'].map(async (path) => {
          const response = await fetch(base + path)
          assert.equal(response.status, 200)
          return [path, await response.text()]
        }),
      )
      http = Object.fromEntries(responses)
      for (const path of new Set(
        Object.values(http).flatMap((code) =>
          [
            ...code.matchAll(
              /['"]([^'"\n]*devup-ui(?:-\d+)?\.css[^'"\n]*)['"]/g,
            ),
          ].flatMap((match) => (match[1] ? [match[1]] : [])),
        ),
      )) {
        const response = await fetch(base + path + '&direct')
        assert.equal(response.status, 200)
        http[path] = await response.text()
      }
    } finally {
      await server.close()
    }
  }
  writeFileSync(
    receipt,
    JSON.stringify(
      {
        assets,
        http,
        events,
        reservations,
        extractions,
        count: wasm.getUnreservedClassSlotCount(),
        css: wasm.getCss(null, false),
        classMap: wasm.exportClassMap(),
        fileMap: wasm.exportFileMap(),
      },
      null,
      2,
    ),
  )
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
