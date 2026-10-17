import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
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
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { DevupUI } from '../../dist/index.mjs'

const require = createRequire(import.meta.url)
const { createRsbuild } = await import(
  pathToFileURL(require.resolve('@rsbuild/core')).href
)
const landing = createRequire(
  join(import.meta.dirname, '../../../../apps/landing/package.json'),
)
const cold = process.argv[2] === 'cold'
const fixture = cold
  ? undefined
  : await realpath(await mkdtemp(join(tmpdir(), 'rsbuild-resolution-watch-')))
const root = cold ? process.argv[3] : join(fixture, 'apps/app')
const palette = resolve(root, '../../packages/palette')
const entry = join(root, 'src/main.js')
const manifest = join(palette, 'package.json')
let server
let result
let next
const pending = []
const nextBuild = () =>
  new Promise((resolveBuild, rejectBuild) => {
    const timer = setTimeout(
      () => rejectBuild(new Error('No real exports-only rebuild')),
      12000,
    )
    timer.unref()
    next = (stats) => {
      clearTimeout(timer)
      next = undefined
      resolveBuild(stats)
    }
    if (pending.length) next(pending.shift())
  })
const state = (stats) => {
  assert.equal(
    stats.hasErrors(),
    false,
    stats.toString({ all: false, errors: true }),
  )
  const compilation = stats.stats
    ? stats.stats[0].compilation
    : stats.compilation
  const module = [...compilation.modules].find(
    (candidate) => candidate.resource === entry,
  )
  const code = module.originalSource().source().toString()
  const selected = code.match(/style\s*=\s*["']([^"']+)["']/)?.[1]
  assert.ok(selected, code)
  const files = Object.keys(compilation.assets).filter((name) =>
    name.endsWith('.css'),
  )
  assert.ok(files.length)
  return {
    manifestWatched: compilation.fileDependencies.has(manifest),
    code,
    selected,
    files,
  }
}
try {
  if (!cold) {
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
  }
  const builder = await createRsbuild({
    cwd: root,
    rsbuildConfig: {
      mode: cold ? 'production' : 'development',
      plugins: [DevupUI()],
      source: { entry: { main: entry } },
      resolve: {
        alias: { '@devup-ui/react': landing.resolve('@devup-ui/react') },
      },
      server: { host: '127.0.0.1', port: 0 },
      output: {
        injectStyles: false,
        minify: false,
        distPath: { root: join(root, cold ? 'cold' : 'out') },
      },
    },
  })
  if (cold) {
    result = await builder.build()
    const files = await import('node:fs/promises').then(({ readdir }) =>
      readdir(join(root, 'cold'), { recursive: true }),
    )
    const css = (
      await Promise.all(
        files
          .filter((name) => name.endsWith('.css'))
          .map((name) => readFile(join(root, 'cold', name), 'utf-8')),
      )
    ).join('\n')
    assert.match(css, /background:\s*(?:blue|#00f)(?:;|\s*\})/)
    assert.doesNotMatch(css, /background:\s*(?:red|#f00)(?:;|\s*\})/)
    console.info(JSON.stringify({ cold: css }))
  } else {
    builder.onAfterDevCompile(({ stats }) =>
      next ? next(stats) : pending.push(stats),
    )
    const initial = nextBuild()
    server = (await builder.startDevServer()).server
    const before = state(await initial)
    assert.ok(before.manifestWatched, `Manifest not registered: ${manifest}`)
    const { port } = server.httpServer.address()
    const css = async (snapshot) =>
      (
        await Promise.all(
          snapshot.files.map(async (file) => {
            const response = await fetch(`http://127.0.0.1:${port}/${file}`, {
              signal: AbortSignal.timeout(10000),
            })
            assert.equal(response.status, 200)
            return response.text()
          }),
        )
      ).join('\n')
    const beforeCss = await css(before)
    assert.match(
      beforeCss,
      new RegExp(
        `\\.${before.selected}\\s*\\{[^}]*background:\\s*red(?:;|\\s*\\})`,
      ),
    )
    const rebuilt = nextBuild()
    const replacement = join(fixture, 'manifest-replacement.json')
    await writeFile(replacement, '{"name":"palette","exports":"./blue.js"}')
    await rename(replacement, manifest)
    let after = state(await rebuilt)
    for (let attempt = 0; attempt < 3; attempt += 1) {
      if (after.selected !== before.selected) break
      after = state(await nextBuild())
    }
    const afterCss = await css(after)
    assert.notEqual(after.selected, before.selected)
    assert.match(
      afterCss,
      new RegExp(
        `\\.${after.selected}\\s*\\{[^}]*background:\\s*(?:blue|#00f)(?:;|\\s*\\})`,
      ),
    )
    await server.close()
    server = undefined
    const built = spawnSync('node', [import.meta.filename, 'cold', root], {
      encoding: 'utf-8',
    })
    assert.equal(built.status, 0, built.stderr)
    console.info(
      JSON.stringify({
        before: beforeCss,
        after: afterCss,
        selected: after.selected,
        cold: built.stdout,
      }),
    )
  }
} finally {
  await server?.close()
  await result?.close()
  if (fixture) await rm(fixture, { recursive: true, force: true })
}
