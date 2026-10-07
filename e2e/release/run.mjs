import assert from 'node:assert/strict'
import {
  mkdir,
  mkdtemp,
  readdir,
  readFile,
  rm,
  writeFile,
} from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { prepareConsumer } from './consumer.mjs'
import { runGuarded } from './guarded-process.mjs'

const repository = fileURLToPath(new URL('../../', import.meta.url))
const root = await mkdtemp(
  join(
    process.env.RELEASE_FIXTURE_ROOT ?? process.env.RUNNER_TEMP ?? tmpdir(),
    'release-compat-',
  ),
)
const measurements = []
const findings = []
const bun = process.env.BUN_BINARY ?? 'bun'
const packages = [
  'wasm',
  'plugin-utils',
  'react',
  'vite-plugin',
  'rsbuild-plugin',
  'webpack-plugin',
  'next-plugin',
  'bun-plugin',
]
const tarballs = {}

const run = (command, args, options) => {
  let output = ''
  return runGuarded(command, args, {
    ...options,
    onOutput: (chunk) => {
      output = (output + chunk).slice(-8192)
    },
    onResult: (result) => measurements.push({ ...result, output }),
  })
}

async function buildAndCheck(name, directory, singleCss) {
  const env = {
    RELEASE_SINGLE_CSS: String(Number(singleCss)),
    NEXT_TELEMETRY_DISABLED: '1',
  }
  const require = createRequire(join(directory, 'package.json'))
  let args
  switch (name) {
    case 'webpack':
      args = [join(directory, 'build-webpack.cjs')]
      break
    case 'vite':
      args = [
        join(dirname(require.resolve('vite/package.json')), 'bin/vite.js'),
        'build',
      ]
      break
    case 'rsbuild':
      args = [
        join(
          dirname(require.resolve('@rsbuild/core/package.json')),
          'bin/rsbuild.js',
        ),
        'build',
      ]
      break
    case 'next-webpack':
      args = [require.resolve('next/dist/bin/next'), 'build', '--webpack']
      break
    case 'next-turbopack':
      args = [require.resolve('next/dist/bin/next'), 'build', '--turbopack']
      break
    default:
      throw new Error(`Unknown fixture ${name}`)
  }
  for (const revision of ['initial', 'edited']) {
    if (revision === 'edited') {
      const source = join(directory, 'src/Fixture.jsx')
      await writeFile(
        source,
        (await readFile(source, 'utf8'))
          .replace('#2468ac', '#ac6824')
          .replace('borderRadius="7px"', ''),
      )
      const theme = join(directory, 'base-theme.json')
      await writeFile(
        theme,
        (await readFile(theme, 'utf8')).replace('#17395b', '#5b3917'),
      )
      const imported = join(directory, 'src/styles.mjs')
      await writeFile(
        imported,
        (await readFile(imported, 'utf8')).replace('9px', '8px'),
      )
    }
    await run(process.execPath, args, { cwd: directory, env })
    const output = join(directory, name.startsWith('next-') ? 'out' : 'dist')
    await run(
      process.execPath,
      [fileURLToPath(new URL('./drive-browser.mjs', import.meta.url)), output],
      { cwd: repository, env: { RELEASE_REVISION: revision } },
    )
  }
  if (name === 'vite' && singleCss) {
    await writeFile(
      join(directory, 'src/styles.mjs'),
      "import { css } from '@devup-ui/react'; export const badge = css({ color: window.name })",
    )
    let rejected = false
    try {
      await run(process.execPath, args, { cwd: directory, env })
    } catch (error) {
      if (!(error instanceof Error)) throw error
      const measurement = measurements.at(-1)
      assert.equal(measurement.code, 1)
      assert.match(
        measurement.output,
        /styles\.mjs:\d+:\d+:.*cannot use.*at build time/,
      )
      measurement.acceptedBuildError = true
      rejected = true
    }
    assert.equal(
      rejected,
      true,
      'unsafe runtime style must fail the production build',
    )
  }
  console.info(`${name} singleCss=${singleCss}: production artifact passed`)
}

try {
  for (const name of packages) {
    const source = join(
      repository,
      name === 'wasm' ? 'bindings/devup-ui-wasm' : `packages/${name}`,
    )
    const destination = join(root, 'packs', name)
    await mkdir(destination, { recursive: true })
    await run(bun, ['pm', 'pack', '--quiet', '--destination', destination], {
      cwd: source,
    })
    const files = await readdir(destination)
    assert.equal(files.length, 1)
    tarballs[name] = `file:${join(destination, files[0]).replaceAll('\\', '/')}`
  }
  const targets = (
    process.env.RELEASE_TARGETS ??
    'vite,rsbuild,webpack,next-webpack,next-turbopack'
  ).split(',')
  for (const name of targets) {
    for (const singleCss of [false, true]) {
      try {
        const directory = await prepareConsumer(name, { root, tarballs }, run)
        for (const generated of ['df', 'dist', '.next', 'out'])
          await rm(join(directory, generated), { recursive: true, force: true })
        await buildAndCheck(name, directory, singleCss)
      } catch (error) {
        if (!(error instanceof Error)) throw error
        findings.push({ fixture: name, singleCss, message: error.message })
        console.error(
          `FINDING ${name} singleCss=${singleCss}: ${error.message}`,
        )
      }
    }
  }
} catch (error) {
  if (!(error instanceof Error)) throw error
  findings.push({ phase: 'preparation', message: error.message })
} finally {
  const summary = {
    commandCount: measurements.length,
    durationMs: measurements.reduce(
      (total, result) => total + result.durationMs,
      0,
    ),
    peakBytes: Math.max(0, ...measurements.map((result) => result.peakBytes)),
    findingCount: findings.length,
  }
  await writeFile(
    join(repository, 'release-results.json'),
    JSON.stringify({ root, summary, findings, measurements }, null, 2),
  )
  console.info('Release compatibility summary:', summary)
}
if (findings.length)
  throw new Error(
    `${findings.length} release compatibility finding(s); see release-results.json`,
  )
await rm(root, { recursive: true, force: true })
