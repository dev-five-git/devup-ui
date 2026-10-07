const { writeFile } = require('node:fs/promises')
const { join } = require('node:path')
const { webpack } = require('next/dist/compiled/webpack/webpack')

async function build() {
  const compiler = webpack(require('./webpack.config.cjs'))
  try {
    const stats = await new Promise((resolve, reject) =>
      compiler.run((error, result) =>
        error ? reject(error) : resolve(result),
      ),
    )
    if (stats.hasErrors())
      throw new Error(stats.toString({ colors: false, errors: true }))
    const assets = stats.toJson({ all: false, assets: true }).assets
    const styles = assets
      .filter(({ name }) => name.endsWith('.css'))
      .map(({ name }) => `<link rel="stylesheet" href="/${name}">`)
      .join('')
    const scripts = assets
      .filter(({ name }) => name.endsWith('.js'))
      .map(({ name }) => `<script defer src="/${name}"></script>`)
      .join('')
    await writeFile(
      join(__dirname, 'dist/index.html'),
      `<!doctype html><html lang="en"><head><meta charset="UTF-8"><title>Release compatibility</title>${styles}</head><body><div id="root"></div>${scripts}</body></html>`,
    )
    console.info(
      `Standalone Webpack ${webpack.version}: production build passed`,
    )
  } finally {
    await new Promise((resolve, reject) =>
      compiler.close((error) => (error ? reject(error) : resolve())),
    )
  }
}

void build().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
