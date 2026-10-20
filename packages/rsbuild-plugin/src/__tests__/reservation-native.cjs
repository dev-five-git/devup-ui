const assert = require('node:assert/strict')
const { writeFileSync } = require('node:fs')
const { basename } = require('node:path')
const { createRsbuild } = require('@rsbuild/core')
const wasm = require('@devup-ui/wasm')
const { put } = require('./reservation-fixture.cjs')
const { runDev } = require('./reservation-dev.cjs')
const { runWatch } = require('./reservation-watch.cjs')

const [root, order, mode, receipt] = process.argv.slice(2)
process.chdir(root)
const events = [],
  extractions = [],
  reservations = []
const originalReserve = wasm.reserveClassSlots
wasm.reserveClassSlots = (...args) => {
  events.push('reserve')
  const css = wasm.getCss(null, false)
  originalReserve(...args)
  assert.equal(wasm.getCss(null, false), css)
  reservations.push({
    rows: args[0],
    flags: args.slice(1),
    count: wasm.getUnreservedClassSlotCount(),
    classMap: wasm.exportClassMap(),
    fileMap: wasm.exportFileMap(),
  })
}
let release
const gate = new Promise((resolve) => {
  release = resolve
})
const originalExtract = wasm.codeExtract
wasm.codeExtract = (...args) => {
  const before = wasm.getUnreservedClassSlotCount()
  const output = originalExtract(...args)
  events.push(`extract:${basename(args[0])}`)
  extractions.push({
    name: args[0],
    code: output.code,
    map: output.map,
    before,
    after: wasm.getUnreservedClassSlotCount(),
  })
  if (basename(args[0]) === `${order[0]}.js`) release()
  return output
}
const { DevupUI } = require('../../dist/index.cjs')
let compilations = 0,
  entries = 0,
  beforeBuild = 0,
  revised = false
const observer = {
  name: 'reservation-observer',
  setup(api) {
    api.onBeforeBuild(() => {
      beforeBuild++
    })
    api.transform(
      { test: /[\\/]app[\\/][AB]\.js$/, order: 'pre' },
      async ({ code, resourcePath }) => {
        if (basename(resourcePath) === `${order[1]}.js`) await gate
        return code
      },
    )
    api.modifyRspackConfig((config) => {
      config.entry = () => {
        entries++
        if (mode === 'error') return { main: { import: ['./src/invalid.js'] } }
        return mode === 'dev' || (mode === 'watch' && !revised)
          ? { main: { import: ['./src/entry.js'] } }
          : { main: { import: ['./app/A.js', './app/B.js'] } }
      }
      config.plugins.push({
        apply(compiler) {
          compiler.options.watchOptions = { poll: 1000 }
          compiler.hooks.thisCompilation.tap('observe', () => {
            compilations++
          })
        },
      })
    })
  },
}

async function main() {
  if (mode === 'dev') {
    put(
      root,
      'src/entry.js',
      'import {css} from "@devup-ui/react"; console.log(css({padding:"19px"})); export const load = () => import("../app/Route.js")',
    )
    put(
      root,
      'app/Route.js',
      'import "./A.js"; import "./B.js"; export const route = "ready"',
    )
  }
  const rsbuild = await createRsbuild({
    cwd: root,
    config: {
      mode: mode === 'dev' ? 'development' : 'production',
      plugins: [
        DevupUI({ singleCss: true, include: ['included-ui'] }),
        observer,
      ],
      source: { entry: { main: './src/entry.js' } },
      output: {
        distPath: { root: 'out' },
        minify: false,
        filenameHash: false,
        sourceMap: { js: 'source-map', css: true },
      },
      performance: { printFileSize: false },
      html: { template: false },
      server: { port: 0, host: '127.0.0.1', printUrls: false },
      dev: {
        lazyCompilation:
          mode === 'dev' ? { entries: false, imports: true } : false,
        hmr: mode === 'dev',
        liveReload: false,
      },
      tools: {
        rspack: {
          cache: false,
          optimization: { concatenateModules: false, minimize: false },
        },
      },
    },
  })
  let compiler, result, lastStats
  try {
    if (mode === 'dev')
      await runDev(rsbuild, { root, order, events, extractions })
    else if (mode === 'build' || mode === 'error') {
      result = await rsbuild.build()
      lastStats = result.stats
    } else {
      compiler = await rsbuild.createCompiler()
      if (mode === 'run')
        lastStats = await new Promise((resolve, reject) =>
          compiler.run((error, stats) =>
            error || stats.hasErrors()
              ? reject(error || new Error(stats.toString()))
              : resolve(stats),
          ),
        )
      else
        lastStats = await runWatch(compiler, {
          root,
          order,
          events,
          revise() {
            revised = true
          },
        })
    }
    const modules = Object.fromEntries(
      extractions
        .filter((item) =>
          /[\\/]app[\\/](?:[AB](?:\.css\.ts|\.js)|Dynamic\.jsx)$/.test(
            item.name,
          ),
        )
        .map((item) => [basename(item.name), item.code]),
    )
    assert.equal(Object.keys(modules).length, 5)
    const css = wasm.getCss(null, false)
    assert(css.includes('blue'))
    assert(!css.includes('chartreuse'))
    const assets = lastStats
      ? Object.fromEntries(
          lastStats.compilation
            .getAssets()
            .map((asset) => [asset.name, String(asset.source.source())]),
        )
      : undefined
    writeFileSync(
      receipt,
      JSON.stringify({
        modules,
        css,
        assets,
        classMap: wasm.exportClassMap(),
        fileMap: wasm.exportFileMap(),
        count: wasm.getUnreservedClassSlotCount(),
        events,
        reservations,
        extractions,
        compilations,
        entries,
        beforeBuild,
      }),
    )
  } finally {
    if (compiler)
      await new Promise((resolve, reject) =>
        compiler.close((error) => (error ? reject(error) : resolve())),
      )
    if (result) await result.close()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
