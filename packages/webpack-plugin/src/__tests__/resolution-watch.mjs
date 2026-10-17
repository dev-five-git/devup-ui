import assert from 'node:assert/strict'
import {
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rename,
  rm,
  symlink,
  writeFile,
} from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { DevupUIWebpackPlugin } from '../../dist/index.mjs'

const landing = createRequire(
  join(import.meta.dirname, '../../../../apps/landing/package.json'),
)
const { webpack } = landing('next/dist/compiled/webpack/webpack')
const fixture = await realpath(
  await mkdtemp(join(tmpdir(), 'webpack-resolution-watch-')),
)
const root = join(fixture, 'apps/app')
const palette = join(fixture, 'packages/palette')
const entry = join(root, 'src/main.js')
const manifest = join(palette, 'package.json')
const preserveSymlinks = process.argv[2] === 'preserve'
let compiler
let watching
let next
const pending = []
const nextBuild = () =>
  new Promise((resolveBuild, rejectBuild) => {
    const timer = setTimeout(
      () => rejectBuild(new Error('No real exports-only rebuild')),
      12000,
    )
    timer.unref()
    next = (error, stats) => {
      clearTimeout(timer)
      next = undefined
      error ? rejectBuild(error) : resolveBuild(stats)
    }
    if (pending.length) next(...pending.shift())
  })
const state = async (stats) => {
  assert.equal(
    stats.hasErrors(),
    false,
    stats.toString({ all: false, errors: true }),
  )
  const module = [...stats.compilation.modules].find(
    (candidate) => candidate.resource === entry,
  )
  const code = module.originalSource().source().toString()
  const selected = code.match(/style\s*=\s*["']([^"']+)["']/)?.[1]
  assert.ok(selected, code)
  const css = (
    await Promise.all(
      Object.keys(stats.compilation.assets)
        .filter((name) => name.endsWith('.css'))
        .map((name) =>
          readFile(join(stats.compilation.outputOptions.path, name), 'utf-8'),
        ),
    )
  ).join('\n')
  return { code, selected, css }
}
const configuration = (watch) => ({
  context: root,
  mode: 'development',
  entry: './src/main.js',
  cache: false,
  output: { path: join(root, watch ? 'out' : 'cold') },
  resolve: {
    symlinks: !preserveSymlinks,
    alias: { '@devup-ui/react': landing.resolve('@devup-ui/react') },
  },
  experiments: { css: true },
  module: { rules: [{ test: /\.css$/, type: 'css' }] },
  optimization: { minimize: false },
  plugins: [new DevupUIWebpackPlugin({ watch, singleCss: false })],
})
try {
  await mkdir(join(root, 'src'), { recursive: true })
  await mkdir(join(root, 'node_modules'), { recursive: true })
  await mkdir(palette, { recursive: true })
  await symlink(palette, join(root, 'node_modules/palette'), 'junction')
  await writeFile(join(root, 'tsconfig.json'), '{}')
  await writeFile(manifest, '{"name":"palette","exports":"./red.js"}')
  await writeFile(join(palette, 'red.js'), "export const color='red'")
  await writeFile(join(palette, 'blue.js'), "export const color='blue'")
  await writeFile(
    entry,
    "import {color} from 'palette'; import {css} from '@devup-ui/react'; export const style=css({bg:color})",
  )
  compiler = webpack(configuration(true))
  const initial = nextBuild()
  watching = compiler.watch({}, (error, stats) =>
    next ? next(error, stats) : pending.push([error, stats]),
  )
  const beforeStats = await initial
  const before = await state(beforeStats)
  assert.ok(
    before.css.includes(`.${before.selected}{background:red}`),
    before.css,
  )
  const watchedManifest = preserveSymlinks
    ? join(root, 'node_modules/palette/package.json')
    : manifest
  assert.ok(
    beforeStats.compilation.fileDependencies.has(watchedManifest),
    `Manifest not registered: ${watchedManifest}`,
  )
  const rebuilt = nextBuild()
  const replacement = join(fixture, 'manifest-replacement.json')
  await writeFile(replacement, '{"name":"palette","exports":"./blue.js"}')
  await rename(replacement, manifest)
  let after = await state(await rebuilt)
  for (
    let attempt = 0;
    attempt < 3 && after.selected === before.selected;
    attempt += 1
  )
    after = await state(await nextBuild())
  assert.notEqual(after.selected, before.selected)
  assert.ok(
    after.css.includes(`.${after.selected}{background:blue}`),
    after.css,
  )
  await new Promise((done, reject) =>
    watching.close((error) => (error ? reject(error) : done())),
  )
  watching = undefined
  await new Promise((done, reject) =>
    compiler.close((error) => (error ? reject(error) : done())),
  )
  compiler = webpack(configuration(false))
  const coldStats = await new Promise((done, reject) =>
    compiler.run((error, stats) => (error ? reject(error) : done(stats))),
  )
  const cold = await state(coldStats)
  assert.ok(cold.css.includes('background:blue'), cold.css)
  assert.ok(!cold.css.includes('background:red'), cold.css)
  console.info(
    JSON.stringify({ preserveSymlinks, watchedManifest, before, after, cold }),
  )
} finally {
  if (watching)
    await new Promise((done, reject) =>
      watching.close((error) => (error ? reject(error) : done())),
    )
  if (compiler)
    await new Promise((done, reject) =>
      compiler.close((error) => (error ? reject(error) : done())),
    )
  await rm(fixture, { recursive: true, force: true })
}
