const { writeFileSync } = require('node:fs')
const { basename } = require('node:path')
/** @type {typeof import('@rsbuild/core')} */
const { createRsbuild } = require('@rsbuild/core')
const { batches, extracts, gate, state } = require('./audit-engine.cjs')
const { manager, changeTarget } = require('./audit-manager.cjs')
const [root = '', mode = '', order = '', receipt = ''] = process.argv.slice(2)
process.chdir(root)

async function native() {
  const compiled = '../../dist/index.cjs'
  /** @type {typeof import('../../dist/plugin')} */
  const { DevupUI } = require(compiled)
  /** @type {unknown[]} */
  const entries = []
  /** @type {unknown[]} */
  const capturedEntries = []
  let compilations = 0
  let beforeBuild = 0
  /** @type {import('@rsbuild/core').RsbuildPlugin} */
  const observer = {
    name: 'audit-custody',
    setup(api) {
      api.onBeforeBuild(() => {
        beforeBuild++
      })
      api.transform(
        { test: /[\\/](?:outside|dist)[\\/][AB]\.js$/, order: 'pre' },
        async ({ code, resourcePath }) => {
          if (basename(resourcePath) === `${order[1]}.js`) await gate
          return code
        },
      )
      api.modifyRspackConfig((config) => {
        capturedEntries.push(config.entry)
        config.plugins ??= []
        config.plugins.push({
          /** @param {import('@rsbuild/core').Rspack.Compiler} compiler */
          apply(compiler) {
            entries.push(compiler.options.entry)
            compiler.hooks.thisCompilation.tap('audit', () => {
              compilations++
            })
          },
        })
      })
    },
  }
  const rsbuild = await createRsbuild({
    cwd: root,
    config: {
      mode: 'production',
      plugins:
        mode === 'reachable-native'
          ? []
          : [DevupUI({ singleCss: true }), observer],
      source: { entry: { main: './src/entry.js' } },
      output: {
        distPath: { root: 'out' },
        minify: false,
        filenameHash: false,
        sourceMap: { js: 'source-map', css: true },
      },
      performance: { printFileSize: false },
      tools: {
        rspack(config) {
          config.cache = false
          config.optimization ??= {}
          config.optimization.concatenateModules = false
          config.optimization.minimize = false
          if (mode.startsWith('override'))
            config.entry = {
              main: { import: ['./outside/A.js', './outside/B.js'] },
            }
        },
      },
    },
  })
  /** @type {import('@rsbuild/core').Rspack.Compiler | import('@rsbuild/core').Rspack.MultiCompiler | undefined} */
  let compiler
  /** @type {Awaited<ReturnType<typeof rsbuild.build>> | undefined} */
  let built
  /** @type {unknown[]} */
  const revisions = []
  /** @param {import('@rsbuild/core').Rspack.Stats | import('@rsbuild/core').Rspack.MultiStats} stats */
  function capture(stats) {
    const all = 'stats' in stats ? stats.stats : [stats]
    const assets = Object.fromEntries(
      all.flatMap((item) =>
        item.compilation
          .getAssets()
          .map((asset) => [asset.name, String(asset.source.source())]),
      ),
    )
    revisions.push({
      ...state(),
      assets,
      extracts: [...extracts],
      batches: [...batches],
    })
  }
  try {
    if (
      ['override-build', 'cold', 'unused', 'reachable-native'].includes(mode)
    ) {
      built = await rsbuild.build()
      if (built.stats) capture(built.stats)
    } else {
      compiler = await rsbuild.createCompiler()
      const active = compiler
      const run = () =>
        new Promise((resolve, reject) => {
          active.run((error, stats) => {
            if (error) return reject(error)
            if (!stats || stats.hasErrors())
              return reject(
                new Error(stats?.toString({}) ?? 'Missing native stats'),
              )
            capture(stats)
            resolve(undefined)
          })
        })
      await run()
      if (mode === 'revision') {
        changeTarget(root)
        await run()
      }
    }
    return {
      ok: true,
      ...state(),
      batches,
      extracts,
      capturedEntries,
      entries,
      compilations,
      beforeBuild,
      revisions,
    }
  } catch (error) {
    if (!(error instanceof Error)) throw error
    return {
      ok: false,
      error: error.stack,
      ...state(),
      batches,
      extracts,
      capturedEntries,
      entries,
      compilations,
      beforeBuild,
      revisions,
    }
  } finally {
    const active = compiler
    if (active)
      await new Promise((resolve, reject) =>
        active.close((error) => (error ? reject(error) : resolve(undefined))),
      )
    if (built) await built.close()
  }
}

async function main() {
  let parser
  try {
    parser = require.resolve('oxc-parser', {
      paths: [require.resolve('@devup-ui/plugin-utils')],
    })
  } catch (error) {
    if (
      !(error instanceof Error) ||
      !('code' in error) ||
      error.code !== 'MODULE_NOT_FOUND'
    )
      throw error
    parser = null
  }
  const result = mode === 'manager' ? manager(root) : await native()
  writeFileSync(receipt, JSON.stringify({ parser, ...result }))
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
