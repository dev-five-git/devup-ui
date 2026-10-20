import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  cp,
  mkdir,
  mkdtemp,
  readdir,
  readFile,
  writeFile,
} from 'node:fs/promises'
import { get } from 'node:https'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repository = fileURLToPath(new URL('../../../../../', import.meta.url))
const fixtures = fileURLToPath(new URL('./fixtures/', import.meta.url))
const packages = [
  'wasm',
  'plugin-utils',
  'react',
  'webpack-plugin',
  'next-plugin',
]
const tools = [
  'react',
  'react-dom',
  'next',
  '@typescript/typescript6',
  'typescript',
  'csstype-extra',
  '@types/react',
  '@types/react-dom',
  '@playwright/test',
]

export function command(binary, args, options) {
  console.info(JSON.stringify({ binary, args, cwd: options.cwd }))
  execFileSync(binary, args, {
    ...options,
    stdio: 'inherit',
    timeout: 600_000,
    windowsHide: true,
  })
}

async function latest(name) {
  const url = `https://registry.npmjs.org/${name.replace('/', '%2f')}/latest`
  const manifest = await new Promise((resolveManifest, reject) => {
    const request = get(url, { timeout: 30_000 }, (response) => {
      if (response.statusCode !== 200) {
        response.resume()
        reject(new Error(`${url}: HTTP ${response.statusCode}`))
        return
      }
      let text = ''
      response.setEncoding('utf8')
      response.on('data', (chunk) => {
        text += chunk
      })
      response.on('error', reject)
      response.on('end', () => {
        try {
          resolveManifest(JSON.parse(text))
        } catch (error) {
          reject(error)
        }
      })
    })
    request.on('error', reject)
    request.on('timeout', () => request.destroy(new Error(`${url}: timeout`)))
  })
  if (manifest.name !== name || typeof manifest.version !== 'string')
    throw new Error(`Invalid latest manifest for ${name}`)
  return {
    version: manifest.version,
    url,
    resolvedAt: new Date().toISOString(),
  }
}

export async function prepareRun(root) {
  const versions = {}
  for (const name of tools) versions[name] = await latest(name)
  const tarballs = {}
  const hashes = []
  const bun = process.env.BUN_BINARY || 'bun'
  for (const name of packages) {
    const source = join(
      repository,
      name === 'wasm' ? 'bindings/devup-ui-wasm' : `packages/${name}`,
    )
    const destination = join(root, 'packs', name)
    await mkdir(destination, { recursive: true })
    command(
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
    const files = (await readdir(destination)).filter((file) =>
      file.endsWith('.tgz'),
    )
    if (files.length !== 1) throw new Error(`Expected one tarball for ${name}`)
    const path = join(destination, files[0])
    tarballs[`@devup-ui/${name}`] = `file:${path.replaceAll('\\', '/')}`
    hashes.push({
      name,
      source,
      path,
      sha256: createHash('sha256')
        .update(await readFile(path))
        .digest('hex'),
    })
  }
  const evidence = {
    node: process.version,
    bunVersion: execFileSync(bun, ['--version'], { encoding: 'utf8' }).trim(),
    versions,
    tarballs,
    hashes,
    repository,
  }
  await writeFile(
    join(root, 'resolution.json'),
    JSON.stringify(evidence, null, 2),
  )
  return { root, bun, ...evidence }
}

export async function prepareConsumer(context, target, singleCss) {
  const directory = await mkdtemp(
    join(context.root, `${target}-${singleCss ? 'single' : 'per-file'}-`),
  )
  await mkdir(join(directory, 'src'))
  for (const name of ['Fixture.jsx', 'styles.mjs', 'main.jsx'])
    await cp(join(fixtures, name), join(directory, 'src', name))
  for (const name of ['devup.json', 'base-theme.json'])
    await cp(join(fixtures, name), join(directory, name))
  const dependencies = Object.fromEntries(
    Object.entries(context.versions)
      .filter(([name]) => name !== '@playwright/test')
      .map(([name, value]) => [name, value.version]),
  )
  Object.assign(dependencies, context.tarballs)
  await writeFile(
    join(directory, 'package.json'),
    JSON.stringify(
      {
        name: 'extends-theme-regression',
        private: true,
        type: 'module',
        dependencies,
        overrides: dependencies,
      },
      null,
      2,
    ),
  )
  switch (target) {
    case 'webpack':
    case 'webpack-esm':
      for (const name of [
        'webpack.config.cjs',
        'build-webpack.cjs',
        'jsx-loader.cjs',
      ])
        await cp(join(fixtures, name), join(directory, name))
      if (target === 'webpack-esm')
        for (const name of ['webpack.config.mjs', 'build-webpack.mjs'])
          await cp(join(fixtures, name), join(directory, name))
      break
    case 'next-webpack':
      await mkdir(join(directory, 'app'))
      for (const name of ['page.jsx', 'layout.jsx'])
        await cp(join(fixtures, name), join(directory, 'app', name))
      await cp(
        join(fixtures, 'next.config.mjs'),
        join(directory, 'next.config.mjs'),
      )
      break
    default:
      throw new Error(`Unknown target ${target}`)
  }
  command(context.bun, ['install'], { cwd: directory })
  const require = createRequire(join(directory, 'package.json'))
  for (const [name, expected] of Object.entries(dependencies)) {
    if (name.startsWith('@devup-ui/')) continue
    const actual = JSON.parse(
      await readFile(
        join(directory, 'node_modules', name, 'package.json'),
        'utf8',
      ),
    )
    if (actual.version !== expected)
      throw new Error(`Resolution mismatch for ${name}`)
  }
  const args =
    target === 'next-webpack'
      ? [require.resolve('next/dist/bin/next'), 'build', '--webpack']
      : [
          join(
            directory,
            target === 'webpack-esm'
              ? 'build-webpack.mjs'
              : 'build-webpack.cjs',
          ),
        ]
  return {
    directory,
    args,
    target,
    singleCss,
    output: join(directory, target === 'next-webpack' ? 'out' : 'dist'),
  }
}

export function buildConsumer(consumer) {
  command(process.execPath, consumer.args, {
    cwd: consumer.directory,
    env: {
      ...process.env,
      RELEASE_SINGLE_CSS: consumer.singleCss ? '1' : '0',
      NEXT_TELEMETRY_DISABLED: '1',
    },
  })
}

export function playwrightRequire() {
  return createRequire(
    join(
      resolve(process.env.DEVUP_PLAYWRIGHT_ROOT || repository),
      'package.json',
    ),
  )
}
