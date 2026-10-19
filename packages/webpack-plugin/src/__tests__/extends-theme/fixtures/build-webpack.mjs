import { writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import config from './webpack.config.mjs'

const require = createRequire(import.meta.url)
const { webpack } = require('next/dist/compiled/webpack/webpack')
const compiler = webpack(config)
try {
  const stats = await new Promise((resolve, reject) =>
    compiler.run((error, result) => (error ? reject(error) : resolve(result))),
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
    join(import.meta.dirname, 'dist/index.html'),
    `<!doctype html><html lang="en"><head><meta charset="UTF-8"><title>Release compatibility</title>${styles}</head><body><div id="root"></div>${scripts}</body></html>`,
  )
  console.info(
    `Standalone direct ESM Webpack ${webpack.version}: production build passed`,
  )
} finally {
  await new Promise((resolve, reject) =>
    compiler.close((error) => (error ? reject(error) : resolve())),
  )
}
