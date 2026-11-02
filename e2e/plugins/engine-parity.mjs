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
import { createServer as createPortLease } from 'node:net'
import { tmpdir } from 'node:os'
import { isAbsolute, join, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  assertRawParity,
  assertRsbuildDist,
  assertRsbuildParity,
  assertViteParity,
  assertWebpackDist,
  collectViteCases,
  completedReceipt,
  errorEvidence,
  viteRealpath,
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

async function viteCells(context) {
  const { root, relocated, link, bun } = context
  const lease = createPortLease()
  const port = await new Promise((resolve, reject) => {
    lease.once('error', reject)
    lease.listen(0, '127.0.0.1', () => {
      const address = lease.address()
      assert(address && typeof address === 'object')
      lease.close((error) => (error ? reject(error) : resolve(address.port)))
    })
  })
  const summary = []
  for (const runtime of [process.execPath, bun]) {
    const name = runtime === bun ? 'bun' : 'node'
    const cells = []
    for (const css of ['per-file', 'single']) {
      for (const format of ['require', 'import']) {
        for (const mode of ['default', 'native', 'wasm'])
          cells.push(['lifecycle', mode, format, css])
        cells.push(['errors', 'native', format, css])
      }
      for (const mode of ['native', 'wasm']) {
        for (const scenario of ['repeat', 'opposite'])
          cells.push([scenario, mode, 'import', css])
      }
    }
    for (const format of ['require', 'import']) {
      cells.push(['guards', 'wasm', format, 'per-file'])
      for (const mode of ['default', 'native', 'wasm'])
        cells.push(['late', mode, format, 'per-file'])
    }
    cells.push(['guards', 'wasm', 'require', 'alias'])
    const receipts = new Map()
    const completed = new Set()
    const results = await collectViteCases(
      cells.map((cell) => cell.join('-')),
      async (id) => {
        const cell = cells.find((cell) => cell.join('-') === id)
        const base = join(root, `${name}-${id}`)
        const destination = `${base}.json`
        try {
          const args = [
            join(link, 'engine-consumer.mjs'),
            ...cell.slice(0, 3),
            cell[3] === 'alias' ? 'per-file' : cell[3],
            destination,
            'vite',
          ]
          run(runtime, args, {
            cwd: relocated,
            evidenceBase: base,
            env: { ...process.env, DEVUP_ENGINE_VITE_PORT: String(port) },
          })
        } finally {
          if (existsSync(destination))
            receipts.set(id, JSON.parse(readFileSync(destination, 'utf8')))
        }
        assert(
          receipts.has(id),
          'Every alias/cell CLI must execute and save its receipt',
        )
        if (cell[0] === 'guards') assert.equal(receipts.get(id).guards, true)
        completed.add(id)
        return destination
      },
    )
    const comparisons = cells
      .filter(([scenario]) =>
        ['lifecycle', 'repeat', 'opposite'].includes(scenario),
      )
      .map((cell) => cell.join('-'))
      .filter((id) => !id.startsWith('lifecycle-default-require-'))
    const compared = await collectViteCases(comparisons, async (id) => {
      const [scenario, mode, , css] = cells.find(
        (cell) => cell.join('-') === id,
      )
      const expected = completedReceipt(
        receipts,
        completed,
        scenario === 'lifecycle'
          ? `lifecycle-default-require-${css}`
          : `lifecycle-${mode}-import-${css}`,
      )
      assertViteParity(
        expected,
        completedReceipt(receipts, completed, id),
        scenario === 'opposite' && css === 'single',
      )
    })
    const index = (rows, group) =>
      rows.map((result) => {
        const diagnostic = join(
          root,
          `${name}-${group}-${result.name}.diagnostic.json`,
        )
        if (!result.ok)
          writeFileSync(diagnostic, JSON.stringify(result.error, null, 2))
        return {
          name: result.name,
          ok: result.ok,
          receipt: result.value,
          partialReceipt:
            !result.ok && receipts.has(result.name)
              ? join(root, `${name}-${result.name}.json`)
              : undefined,
          diagnostic: result.ok ? undefined : diagnostic,
        }
      })
    summary.push({
      runtime: name,
      passed: [...results, ...compared].filter((cell) => cell.ok).length,
      failed: [...results, ...compared].filter((cell) => !cell.ok).length,
      results: index(results, 'execute'),
      comparisons: index(compared, 'compare'),
    })
    writeFileSync(
      join(root, 'vite-summary.json'),
      JSON.stringify(summary, null, 2),
    )
  }
  const cuts = join(root, 'dev-cuts')
  try {
    run(
      process.execPath,
      [join(link, 'engine-dev-cuts.mjs'), 'matrix', root, String(port)],
      {
        cwd: relocated,
        evidenceBase: cuts,
      },
    )
    summary.push({ runtime: 'node', cuts: true })
  } catch (error) {
    summary.push({ runtime: 'node', cuts: false, error: errorEvidence(error) })
  }
  writeFileSync(
    join(root, 'vite-summary.json'),
    JSON.stringify(summary, null, 2),
  )
  assert(
    summary.every(
      (row) =>
        row.cuts ??
        [...row.results, ...row.comparisons].every((cell) => cell.ok),
    ),
    'Vite collect-all diagnostics failed; inspect vite-summary.json and raw cell evidence before one coordinator batch decision',
  )
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
      `file:${(adapter === 'vite' ? viteRealpath : realpathSync)(join(destination, packs[0])).split(sep).join('/')}`
  }
  const assertDist =
    adapter === 'rsbuild' || adapter === 'vite'
      ? assertRsbuildDist
      : assertWebpackDist
  assertDist(join(repository, `packages/${adapter}-plugin/dist`))
  const bundler =
    adapter === 'vite'
      ? 'vite'
      : adapter === 'rsbuild'
        ? '@rsbuild/core'
        : 'webpack'
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
  if (adapter === 'vite') {
    cpSync(
      new URL('./engine-dev-cuts.mjs', import.meta.url),
      join(relocated, 'engine-dev-cuts.mjs'),
    )
    await viteCells({ root, relocated, link, bun })
    return root
  }
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
export const viteParity = () => webpackParity('vite')

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const adapter = process.argv[2] ?? 'webpack'
  assert(['webpack', 'rsbuild', 'vite'].includes(adapter))
  await webpackParity(adapter)
}
