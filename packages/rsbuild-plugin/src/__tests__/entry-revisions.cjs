const { writeFileSync } = require('node:fs')
const { basename } = require('node:path')
/** @type {typeof import('@rsbuild/core')} */
const { createRsbuild } = require('@rsbuild/core')
const { batches, extracts, gate, state } = require('./audit-engine.cjs')
const [root = '', mode = '', order = '', receipt = ''] = process.argv.slice(2)
process.chdir(root)

async function main() {
  const compiled = '../../dist/index.cjs'
  /** @type {typeof import('../../dist/plugin')} */
  const { DevupUI } = require(compiled)
  let revision = 0
  let beforeBuild = 0
  let compilations = 0
  /** @type {unknown[]} */
  const entries = []
  /** @type {import('@rsbuild/core').RsbuildPlugin} */
  const observer = {
    name: 'entry-revision-custody',
    setup(api) {
      api.onBeforeBuild(() => {
        beforeBuild++
      })
      api.transform(
        { test: /[\\/]outside[\\/][AB]\.js$/, order: 'pre' },
        async ({ code, resourcePath }) => {
          if (basename(resourcePath) === `${order[1]}.js`) await gate
          return code
        },
      )
      api.onAfterCreateCompiler(({ compiler }) => {
        const all = 'compilers' in compiler ? compiler.compilers : [compiler]
        entries.push(all.map((item) => item.options.entry))
        for (const item of all)
          item.hooks.thisCompilation.tap('entry-revision-custody', () => {
            compilations++
          })
      })
    },
  }
  const rsbuild = await createRsbuild({
    cwd: root,
    config: {
      mode: 'production',
      plugins: [DevupUI({ singleCss: true }), observer],
      source: { entry: { main: './src/entry.js' } },
      environments: {
        first: { output: { distPath: { root: 'out-first' } } },
        second: { output: { distPath: { root: 'out-second' } } },
      },
      output: {
        minify: false,
        filenameHash: false,
        sourceMap: { js: 'source-map', css: true },
      },
      performance: { printFileSize: false },
      tools: {
        rspack(config, { environment }) {
          config.cache = false
          config.optimization ??= {}
          config.optimization.concatenateModules = false
          config.optimization.minimize = false
          config.entry = {
            main: {
              import: [
                environment.name === 'first'
                  ? revision === 0
                    ? './outside/A.js'
                    : './revised/A.js'
                  : './outside/B.js',
              ],
            },
          }
        },
      },
    },
  })
  /** @type {unknown[]} */
  const revisions = []
  for (; revision < 2; revision++) {
    const compiler = await rsbuild.createCompiler()
    const batchStart = batches.length
    const extractStart = extracts.length
    try {
      await new Promise((resolve, reject) => {
        compiler.run((error, stats) => {
          if (error) return reject(error)
          if (!stats || stats.hasErrors())
            return reject(new Error(stats?.toString({}) ?? 'Missing stats'))
          const all = 'stats' in stats ? stats.stats : [stats]
          revisions.push({
            ...state(),
            batches: batches.slice(batchStart),
            extracts: extracts.slice(extractStart),
            assets: all.map((item) =>
              Object.fromEntries(
                item.compilation
                  .getAssets()
                  .map((asset) => [asset.name, String(asset.source.source())]),
              ),
            ),
          })
          resolve(undefined)
        })
      })
    } finally {
      await new Promise((resolve, reject) =>
        compiler.close((error) => (error ? reject(error) : resolve(undefined))),
      )
    }
  }
  writeFileSync(
    receipt,
    JSON.stringify({ mode, entries, revisions, beforeBuild, compilations }),
  )
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
