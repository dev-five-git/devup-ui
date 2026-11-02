import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync, rmSync, statSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { targets } from '../../e2e/binding/package/matrix.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '../..')
const args = process.argv.slice(2)
assert.ok(
  args.length === 2 || (args.length === 3 && args[2] === '--zigbuild'),
  'Usage: node build.mjs --target <Rust triple> [--zigbuild]',
)
assert.equal(args[0], '--target')
const row = targets.find(({ target }) => target === args[1])
assert.ok(row, `Unsupported build target: ${args[1]}`)
const zig = args[2] === '--zigbuild'
assert.ok(!zig || row.os === 'linux', 'zigbuild is only supported for Linux')
const targetDirectory = resolve(root, process.env.CARGO_TARGET_DIR || 'target')
const library =
  row.os === 'win32'
    ? 'devup_ui_binding.dll'
    : row.os === 'darwin'
      ? 'libdevup_ui_binding.dylib'
      : 'libdevup_ui_binding.so'
const source = join(targetDirectory, row.target, 'native', library)
const destination = join(
  here,
  `../devup-ui-binding-${row.suffix}/binding.${row.suffix}.node`,
)
// Remove both images before Cargo: a failed build cannot leave a shippable stale copy.
rmSync(destination, { force: true })
rmSync(source, { force: true })
const target = zig && row.libc === 'glibc' ? `${row.target}.2.28` : row.target
const env = { ...process.env, CARGO_BUILD_JOBS: '2' }
if (row.libc === 'musl') {
  env.RUSTFLAGS = `${env.RUSTFLAGS || ''} -C target-feature=-crt-static`.trim()
}
execFileSync(
  'cargo',
  [
    zig ? 'zigbuild' : 'build',
    '--locked',
    '--profile',
    'native',
    '--target',
    target,
    '--jobs',
    '2',
    '-p',
    'devup-ui-binding',
  ],
  { cwd: root, env, stdio: 'inherit' },
)
assert.ok(statSync(source).size > 0, 'Cargo must produce the requested image')
copyFileSync(source, destination)
mkdirSync(join(here, 'pkg'), { recursive: true })
execFileSync(
  process.execPath,
  [join(here, 'script.cjs'), '--public', join(here, 'pkg/index.cjs')],
  { cwd: root, stdio: 'inherit' },
)
console.info(
  JSON.stringify({
    target: row.target,
    profile: 'native',
    source,
    destination,
  }),
)
