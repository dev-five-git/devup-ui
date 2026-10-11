import assert from 'node:assert/strict'
import { dirname, join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { build } from 'bun'

export const webpackFixtureManifest = {
  name: '@devup-ui/webpack-plugin',
  type: 'module',
  exports: {
    '.': './index.cjs',
    './loader': './loader.cjs',
    './css-loader': './css-loader.cjs',
  },
}

export const pluginUtilsFixtureManifest = {
  name: '@devup-ui/plugin-utils',
  type: 'module',
  exports: {
    '.': { import: './index.mjs', require: './index.cjs' },
    './internal/build-admission': './build-admission.cjs',
  },
}

export const nextSourceEntries = [
  ['next-plugin', 'index', 'esm', 'mjs'],
  ['webpack-plugin', 'index', 'cjs', 'cjs'],
  ['webpack-plugin', 'loader', 'cjs', 'cjs'],
  ['webpack-plugin', 'css-loader', 'cjs', 'cjs'],
  ['plugin-utils', 'index', 'esm', 'mjs'],
  ['plugin-utils', 'index', 'cjs', 'cjs'],
  ['plugin-utils', 'build-admission', 'cjs', 'cjs'],
]

export async function bundleSourceEntries(workspace, root, entries) {
  const bundles = []
  for (const [pkg, entry, format, ext] of entries) {
    const source = join(
      workspace,
      'packages',
      pkg,
      'src',
      `${entry}.${entry === 'build-admission' ? 'cts' : 'ts'}`,
    )
    const emitted = join(root, 'node_modules/@devup-ui', pkg, `${entry}.${ext}`)
    const built = await build({
      entrypoints: [source],
      outdir: dirname(emitted),
      naming: `${entry}.${ext}`,
      target: 'node',
      format,
      packages: 'external',
      define: {
        'import.meta.url': JSON.stringify(pathToFileURL(emitted).href),
      },
    })
    assert.ok(built.success, String(built.logs))
    bundles.push({ source, emitted, success: built.success })
  }
  return bundles
}
