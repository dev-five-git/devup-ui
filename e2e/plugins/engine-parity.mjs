import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  realpathSync,
  renameSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { isAbsolute, join, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  assertRawParity,
  assertRsbuildDist,
  assertRsbuildParity,
  assertWebpackDist,
} from './engine-consumer.mjs'
import { packHostEngines, verifyHostInstall } from './engine-fixtures.mjs'

let artifactRoot
let invocation = 0

function run(binary, args, options) {
  console.info(JSON.stringify({ binary, args, cwd: options.cwd }))
  const { evidenceBase, ...execution } = options
  const base = evidenceBase ?? join(artifactRoot, `command-${invocation++}`)
  const result = spawnSync(binary, args, {
    ...execution,
    timeout: 600000,
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  })
  writeFileSync(`${base}.stdout`, result.stdout ?? Buffer.alloc(0))
  writeFileSync(`${base}.stderr`, result.stderr ?? Buffer.alloc(0))
  if (result.error) throw result.error
  assert.equal(result.status, 0, `Command failed; raw streams: ${base}`)
  return result.stdout.toString('utf8')
}

export async function webpackParity(adapter = 'webpack') {
  const repository = fileURLToPath(new URL('../../', import.meta.url))
  const parent = process.env.DEVUP_ENGINE_ARTIFACT_PARENT ?? tmpdir()
  const root = mkdtempSync(join(parent, `devup-${adapter}-engine-`))
  artifactRoot = root
  console.info(`${adapter} engine evidence: ${root}`)
  const bun = process.env.BUN_BINARY ?? 'bun'
  const context = { repository, root, bun }
  const { host, tarballs } = await packHostEngines(context, run)
  for (const name of ['wasm', 'plugin-utils', 'react', `${adapter}-plugin`]) {
    const source = join(
      repository,
      name === 'wasm' ? 'bindings/devup-ui-wasm' : `packages/${name}`,
    )
    const destination = join(root, 'packs', name)
    mkdirSync(destination, { recursive: true })
    run(
      bun,
      [
        'pm',
        'pack',
        '--ignore-scripts',
        '--quiet',
        '--destination',
        destination,
      ],
      { cwd: source },
    )
    const packs = readdirSync(destination).filter((file) =>
      file.endsWith('.tgz'),
    )
    assert.equal(packs.length, 1)
    tarballs[`@devup-ui/${name}`] =
      `file:${realpathSync(join(destination, packs[0])).split(sep).join('/')}`
  }
  const assertDist =
    adapter === 'rsbuild' ? assertRsbuildDist : assertWebpackDist
  assertDist(join(repository, `packages/${adapter}-plugin/dist`))
  const bundler = adapter === 'rsbuild' ? '@rsbuild/core' : 'webpack'
  const versions = {}
  for (const name of [bundler, 'react', 'csstype-extra']) {
    const response = await fetch(`https://registry.npmjs.org/${name}/latest`, {
      signal: AbortSignal.timeout(30000),
    })
    assert.equal(response.status, 200)
    const manifest = await response.json()
    assert.equal(manifest.name, name)
    versions[name] = manifest.version
  }
  const directory = join(root, 'consumer')
  mkdirSync(directory)
  writeFileSync(
    join(directory, 'package.json'),
    JSON.stringify(
      {
        name: `${adapter}-engine-consumer`,
        private: true,
        type: 'module',
        dependencies: {
          ...versions,
          [`@devup-ui/${adapter}-plugin`]:
            tarballs[`@devup-ui/${adapter}-plugin`],
          '@devup-ui/react': tarballs['@devup-ui/react'],
        },
        overrides: tarballs,
      },
      null,
      2,
    ),
  )
  run(
    bun,
    ['install', '--ignore-scripts', '--linker=isolated', ...host.installArgs],
    { cwd: directory },
  )
  const beforeRelocation = verifyHostInstall({ directory, tarballs, host, bun })
  const relocated = join(root, 'relocated consumer')
  const entryLink = join(directory, 'node_modules', bundler)
  assert(lstatSync(entryLink).isSymbolicLink())
  const target = readlinkSync(entryLink)
  const reinstall = isAbsolute(target)
  const relocation = {
    intent:
      'Keep a moved installed consumer usable: relative installer links move unchanged; absolute installer links require a frozen reinstall, not a claim of portable absolute junctions.',
    entryLink,
    target,
    branch: reinstall
      ? 'absolute-link-frozen-reinstall'
      : 'relative-link-plain-move',
    beforeRelocation,
  }
  writeFileSync(
    join(root, 'relocation-before.json'),
    JSON.stringify(relocation, null, 2),
  )
  renameSync(directory, relocated)
  const lock = readFileSync(join(relocated, 'bun.lock'))
  writeFileSync(join(root, 'relocation-before.lock'), lock)
  if (reinstall) {
    run(
      bun,
      [
        'install',
        '--frozen-lockfile',
        '--ignore-scripts',
        '--linker=isolated',
        ...host.installArgs,
      ],
      { cwd: relocated },
    )
  }
  const relocatedLock = readFileSync(join(relocated, 'bun.lock'))
  writeFileSync(join(root, 'relocation-after.lock'), relocatedLock)
  assert.deepEqual(relocatedLock, lock)
  const afterRelocation = verifyHostInstall({
    directory: relocated,
    tarballs,
    host,
    bun,
  })
  writeFileSync(
    join(root, 'relocation-after.json'),
    JSON.stringify({ ...relocation, afterRelocation }, null, 2),
  )
  cpSync(
    new URL('./engine-consumer.mjs', import.meta.url),
    join(relocated, 'engine-consumer.mjs'),
  )
  const link = join(root, 'consumer link')
  symlinkSync(
    relocated,
    link,
    process.platform === 'win32' ? 'junction' : 'dir',
  )
  writeFileSync(
    join(root, 'provenance.json'),
    JSON.stringify(
      {
        repository,
        versions,
        host,
        tarballs,
        bun: run(bun, ['--version'], { cwd: root }).trim(),
        node: process.version,
      },
      null,
      2,
    ),
  )
  for (const runtime of [process.execPath, bun]) {
    const runtimeName = runtime === bun ? 'bun' : 'node'
    const aliasControl = join(root, `${runtimeName}-alias-entry-control`)
    const aliasReceipt = `${aliasControl}.json`
    run(
      runtime,
      [
        join(link, 'engine-consumer.mjs'),
        'guards',
        'wasm',
        'require',
        'per-file',
        aliasReceipt,
        adapter,
      ],
      { cwd: relocated, evidenceBase: aliasControl },
    )
    assert(
      existsSync(aliasReceipt),
      'The alias CLI must execute and produce a receipt',
    )
    assert.equal(JSON.parse(readFileSync(aliasReceipt, 'utf8')).guards, true)
    const cell = (scenario, mode, format, css) => {
      const base = join(
        root,
        `${runtimeName}-${scenario}-${mode}-${format}-${css}`,
      )
      const destination = `${base}.json`
      run(
        runtime,
        [
          join(link, 'engine-consumer.mjs'),
          scenario,
          mode,
          format,
          css,
          destination,
          adapter,
        ],
        { cwd: relocated, evidenceBase: base },
      )
      return JSON.parse(readFileSync(destination, 'utf8'))
    }
    for (const scenario of [
      'build',
      'additional',
      'watch',
      ...(adapter === 'rsbuild' ? ['dev'] : []),
    ]) {
      for (const css of ['per-file', 'single']) {
        let expected
        for (const format of ['require', 'import']) {
          for (const mode of ['default', 'native', 'wasm']) {
            const actual = cell(scenario, mode, format, css)
            if (expected === undefined) expected = actual
            else if (adapter === 'rsbuild')
              assertRsbuildParity(expected, actual)
            else assertRawParity(expected, actual)
          }
        }
      }
    }
    for (const format of ['require', 'import']) {
      for (const mode of ['default', 'native', 'wasm']) {
        cell('late', mode, format, 'per-file')
      }
      cell('guards', 'wasm', format, 'per-file')
      if (adapter === 'webpack')
        cell('wrong-loader', 'default', format, 'per-file')
      for (const css of ['per-file', 'single']) {
        cell('errors', 'native', format, css)
      }
    }
  }
  console.info(`${adapter} engine evidence: ${root}`)
  return root
}

export const rsbuildParity = () => webpackParity('rsbuild')

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const adapter = process.argv[2] ?? 'webpack'
  assert(['webpack', 'rsbuild'].includes(adapter))
  await webpackParity(adapter)
}
