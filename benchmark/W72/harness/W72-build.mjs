import { execFileSync } from 'node:child_process'
import { cpSync, existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = process.env.W72_ROOT ?? process.cwd()
const out = process.env.W72_OUT ?? dirname(fileURLToPath(import.meta.url))
const variant = process.argv[2]
const run = (command, args, options = {}) =>
  execFileSync(command, args, { stdio: 'inherit', cwd: root, ...options })
process.env.CARGO_BUILD_JOBS = '2'
delete process.env.CI
if (
  process.env.RUST_MIN_STACK ||
  /stack-size|STACK:/.test(process.env.RUSTFLAGS ?? '')
)
  throw new Error('Default stack required')
switch (variant) {
  case 'save-W1':
    cpSync(join(root, 'bindings/devup-ui-wasm/pkg'), join(out, 'W72-W1'), {
      recursive: true,
    })
    break
  case 'W2': {
    const dir = join(out, 'W72-W2')
    run(
      'wasm-pack',
      [
        'build',
        '--target',
        'nodejs',
        '--out-name',
        'index',
        '--out-dir',
        dir,
        '--no-opt',
      ],
      {
        cwd: join(root, 'bindings/devup-ui-wasm'),
        env: { ...process.env, CARGO_PROFILE_RELEASE_OPT_LEVEL: '3' },
      },
    )
    const optimizer =
      process.env.W72_WASM_OPT ??
      join(
        process.env.LOCALAPPDATA,
        '.wasm-pack/wasm-opt-cdcb9e877b68d02e/bin/wasm-opt.exe',
      )
    run(optimizer, [
      join(dir, 'index_bg.wasm'),
      '-o',
      join(dir, 'W72-optimized.wasm'),
      '-O3',
      '--enable-bulk-memory',
      '--enable-nontrapping-float-to-int',
      '--enable-sign-ext',
    ])
    cpSync(join(dir, 'W72-optimized.wasm'), join(dir, 'index_bg.wasm'))
    writeFileSync(join(dir, 'package.json'), '{}')
    break
  }
  case 'N1':
  case 'Nz': {
    const dir = join(root, 'benchmark/W72/native')
    const lock = join(dir, 'Cargo.lock')
    if (!existsSync(lock)) cpSync(join(root, 'Cargo.lock'), lock)
    run(
      'cargo',
      ['build', '--manifest-path', join(dir, 'Cargo.toml'), '--release'],
      {
        env: {
          ...process.env,
          CARGO_PROFILE_RELEASE_OPT_LEVEL: variant === 'Nz' ? 'z' : '3',
        },
      },
    )
    const output = join(out, `W72-${variant}`)
    mkdirSync(output, { recursive: true })
    cpSync(
      join(dir, 'target/release/devup_ui_w72_native.dll'),
      join(output, 'devup_ui_w72_native.node'),
    )
    cpSync(join(dir, 'index.cjs'), join(output, 'index.cjs'))
    break
  }
  case 'packages': {
    for (const name of [
      'plugin-utils',
      'react',
      'webpack-plugin',
      'eslint-plugin',
      'vite-plugin',
      'next-plugin',
      'rsbuild-plugin',
      'bun-plugin',
      'components',
      'reset-css',
    ]) {
      run('bun', ['run', 'build'], { cwd: join(root, 'packages', name) })
    }
    run('bun', ['run', 'build'], { cwd: join(root, 'apps/vite-lib') })
    break
  }
  case 'node-latest': {
    const metadata = await (
      await fetch('https://registry.npmjs.org/node/latest')
    ).json()
    const version = metadata.version
    const target = join(out, 'W72-node')
    mkdirSync(target, { recursive: true })
    const response = await fetch(
      `https://nodejs.org/dist/v${version}/win-x64/node.exe`,
    )
    if (!response.ok) throw new Error(`Node download: ${response.status}`)
    writeFileSync(
      join(target, 'node.exe'),
      Buffer.from(await response.arrayBuffer()),
    )
    writeFileSync(join(target, 'W72-version.txt'), `${version}\n`)
    break
  }
  default:
    throw new Error(`Unknown variant ${variant}`)
}
