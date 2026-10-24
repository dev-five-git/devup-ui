const assert = require('node:assert/strict')
const { writeFileSync } = require('node:fs')
const { resolve } = require('node:path')
const { fixture, put } = require('./reservation-native-fixture.cjs')
const { observe } = require('./reservation-native-observer.cjs')
const { text } = require('./reservation-native-receipt.cjs')
const root = text(process.argv[2])
const receipt = text(process.argv[3])
process.chdir(root)
const project = fixture(root, 'packages', 'build')
const telemetry = observe()
/** @type {{DevupUI:(options:{distDir:string,cssDir:string,devupFile:string,singleCss:boolean,include:string[]})=>import('vite').PluginOption}} */
const { DevupUI } = require('../../dist/' + 'index.cjs')
/** @type {Record<string,unknown>[]} */
const runs = []
/** @param {string} phase */
function save(phase) {
  writeFileSync(
    receipt,
    JSON.stringify({ phase, runs, telemetry: telemetry.snapshot() }, null, 2),
  )
}
async function main() {
  const { build, createServer, version } = await import('vite')
  for (const color of ['gold', 'teal']) {
    put(
      root,
      'node_modules/accepted/dist/index.js',
      `import {css} from '@devup-ui/react'; export const accepted=css({borderColor:'${color}'})`,
    )
    /** @type {Record<string,unknown>} */
    const run = { color, version }
    runs.push(run)
    /** @returns {import('vite').InlineConfig} */
    const config = () => ({
      root,
      configFile: false,
      logLevel: 'error',
      oxc: { jsx: { runtime: 'classic' } },
      plugins: [
        DevupUI({
          distDir: resolve(root, 'df'),
          cssDir: resolve(root, 'df/devup-ui'),
          devupFile: resolve(root, 'devup.json'),
          singleCss: true,
          include: ['accepted'],
        }),
      ],
      build: { ...project.build, minify: false, write: false },
      server: { host: '127.0.0.1', port: 0, fs: { strict: false } },
      optimizeDeps: { noDiscovery: true, include: [] },
    })
    save('fresh-build-enter')
    const built = await build(config())
    assert(!Array.isArray(built) && 'output' in built)
    const assets = built.output.map((item) =>
      item.type === 'chunk' ? item.code : String(item.source),
    )
    assert(assets.some((asset) => asset.includes(`border-color:${color}`)))
    run.assets = assets
    save('native-restart-enter')
    const server = await createServer(config())
    try {
      await server.listen()
      const address = server.httpServer?.address()
      assert(address && typeof address === 'object')
      const base = `http://127.0.0.1:${address.port}`
      const response = await fetch(
        base + '/node_modules/accepted/dist/index.js',
      )
      assert.equal(response.status, 200)
      const code = await response.text()
      run.code = code
      /** @type {Record<string,string>} */
      const http = {}
      run.http = http
      for (const match of code.matchAll(
        /['"](\/[^'"\n]*devup-ui(?:-\d+)?\.css[^'"\n]*)['"]/g,
      )) {
        assert(match[1])
        const path = match[1] + '&restart'
        const css = await fetch(base + path)
        assert.equal(css.status, 200)
        http[path] = await css.text()
      }
      assert(
        Object.values(http).some((css) =>
          css.includes(`border-color:${color}`),
        ),
      )
      assert.equal(telemetry.snapshot().count, 0)
      save('outcome-before-close')
    } catch (error) {
      run.error =
        error instanceof Error
          ? { message: error.message, stack: error.stack }
          : String(error)
      save('error-before-close')
      throw error
    } finally {
      save('close-enter')
      await server.close()
      run.closed = true
      save('close-exit')
    }
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
