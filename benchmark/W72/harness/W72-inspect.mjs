import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import os from 'node:os'

const versions = {}
for (const name of ['napi', 'napi-derive', 'napi-build']) {
  const response = await fetch(`https://crates.io/api/v1/crates/${name}`, {
    headers: {
      'User-Agent':
        'Devup-UI-W72-binding-benchmark (https://github.com/dev-five-git/devup-ui)',
    },
  })
  if (!response.ok) throw new Error(`${name}: ${response.status}`)
  versions[name] = (await response.json()).crate.max_stable_version
}
for (const name of ['@napi-rs/cli', '@napi-rs/wasm-runtime', 'node', 'bun']) {
  const response = await fetch(`https://registry.npmjs.org/${name}/latest`)
  if (!response.ok) throw new Error(`${name}: ${response.status}`)
  versions[name] = (await response.json()).version
}
const run = (program, args) =>
  execFileSync(program, args, { encoding: 'utf8' }).trim()
const result = {
  at: new Date().toISOString(),
  versions,
  platform: os.platform(),
  release: os.release(),
  architecture: os.arch(),
  cpus: os.cpus().map((cpu) => cpu.model),
  ramBytes: os.totalmem(),
  freeBytes: os.freemem(),
  bun: run('bun', ['--version']),
  node: process.version,
  rust: run('rustc', ['--version', '--verbose']),
  wasmPack: run('wasm-pack', ['--version']),
  targets: run('rustup', ['target', 'list', '--installed']),
  env: Object.fromEntries(
    ['CARGO_TARGET_DIR', 'RUSTFLAGS', 'RUST_MIN_STACK', 'CI'].map((key) => [
      key,
      process.env[key] ?? null,
    ]),
  ),
}
writeFileSync(
  new URL('W72-environment.json', import.meta.url),
  JSON.stringify(result, null, 2),
)
process.stdout.write(JSON.stringify(result, null, 2) + '\n')
