const { mkdirSync, mkdtempSync, rmSync } = require('node:fs')
const { createRequire } = require('node:module')
const { tmpdir } = require('node:os')
const { join, resolve } = require('node:path')

const workspace = resolve(__dirname, '../../../..')
const installed = createRequire(join(workspace, 'apps/landing/package.json'))
const { default: getBaseWebpackConfig } = installed(
  'next/dist/build/webpack-config',
)
const { defaultConfig } = installed('next/dist/server/config-shared')
const { trace } = installed('next/dist/trace')
const bundle = installed('next/dist/compiled/webpack/webpack')

exports.withStockClient = async function withStockClient(scenario, run) {
  const root = mkdtempSync(join(tmpdir(), 'devup-next-stock-alias-'))
  try {
    const directory = join(root, scenario.router)
    mkdirSync(directory)
    const original = await getBaseWebpackConfig(root, {
      buildId: 'stock-alias-regression',
      encryptionKey: 'stock-alias-regression',
      config: {
        ...defaultConfig,
        experimental: { ...defaultConfig.experimental },
      },
      compilerType: 'client',
      dev: scenario.dev,
      entrypoints: {},
      deferredEntrypoints: {},
      pagesDir: scenario.router === 'pages' ? directory : undefined,
      appDir: scenario.router === 'app' ? directory : undefined,
      rewrites: { beforeFiles: [], afterFiles: [], fallback: [] },
      originalRewrites: [],
      originalRedirects: [],
      runWebpackSpan: trace('stock-alias-regression'),
      supportedBrowsers: [],
      previewProps: {
        previewModeId: 'stock-alias-regression',
        previewModeSigningKey: 'stock-alias-regression',
        previewModeEncryptionKey: 'stock-alias-regression',
      },
    })
    const compiler = bundle.webpack(original)
    try {
      return await run({
        compiler,
        params: compiler.newCompilationParams(),
        effectiveConfiguration: compiler.options,
        pipelines: [],
      })
    } finally {
      await new Promise((resolve, reject) =>
        compiler.close((error) => (error ? reject(error) : resolve())),
      )
    }
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}
