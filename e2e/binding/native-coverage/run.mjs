import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { copyFileSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..')
assert.equal(
  process.versions.bun,
  undefined,
  'Run native Rust coverage with Node',
)
assert.ok(!process.env.RUST_MIN_STACK, 'Default Rust stack is required')
assert.doesNotMatch(process.env.RUSTFLAGS ?? '', /stack-size|STACK:/)
const evidence = mkdtempSync(join(tmpdir(), 'devup-native-coverage-'))
const image = join(evidence, 'binding.node')
const entry = join(evidence, 'binding.cjs')
const target = process.env.CARGO_TARGET_DIR
  ? resolve(root, process.env.CARGO_TARGET_DIR)
  : join(root, 'target')
const library = {
  win32: 'devup_ui_binding.dll',
  linux: 'libdevup_ui_binding.so',
  darwin: 'libdevup_ui_binding.dylib',
}[process.platform]
assert.ok(library, `Unsupported native coverage host: ${process.platform}`)
const env = { ...process.env, CARGO_BUILD_JOBS: '2' }
delete env.LLVM_PROFILE_FILE
const commands = []
const run = (program, args, childEnv) => {
  const started = performance.now()
  const child = spawnSync(program, args, {
    cwd: root,
    env: childEnv,
    stdio: 'inherit',
  })
  commands.push({
    program,
    args,
    pid: child.pid,
    elapsedMs: performance.now() - started,
    status: child.status,
    signal: child.signal,
    error: child.error?.message,
  })
  if (child.error) throw child.error
  assert.equal(child.signal, null, `${program}: natural exit required`)
  assert.equal(child.status, 0, `${program}: native coverage failed`)
}
const hash = (path) =>
  createHash('sha256').update(readFileSync(path)).digest('hex')
let imageHash
try {
  const buildEnv = {
    ...env,
    RUSTFLAGS: '-C instrument-coverage -C debuginfo=2 -C strip=none',
    LLVM_PROFILE_FILE: join(evidence, 'build-%m-%p.profraw'),
  }
  delete buildEnv.CARGO_ENCODED_RUSTFLAGS
  run(
    'cargo',
    ['build', '--locked', '-p', 'devup-ui-binding', '--jobs', '2'],
    buildEnv,
  )
  copyFileSync(join(target, 'debug', library), image)
  imageHash = hash(image)
  run(
    process.execPath,
    [join(root, 'bindings/devup-ui-binding/script.cjs'), image, entry],
    env,
  )
  const args = [
    'tarpaulin',
    '--out',
    'xml',
    '--out',
    'stdout',
    '--all-targets',
    '--engine',
    'llvm',
    '--timeout',
    '180',
    '--features',
    'devup-ui-binding/native-coverage',
    '--objects',
    image,
  ]
  if (process.env.CI) args.push('--fail-under', '100')
  run('cargo', args, {
    ...env,
    DEVUP_NATIVE_COVERAGE_NODE: process.execPath,
    DEVUP_NATIVE_COVERAGE_ENTRY: entry,
    DEVUP_NATIVE_COVERAGE_OBJECT: image,
    DEVUP_NATIVE_COVERAGE_EVIDENCE: evidence,
    DEVUP_NATIVE_COVERAGE_HOST: join(
      root,
      'e2e/binding/native-coverage/scenarios.mjs',
    ),
  })
  assert.equal(hash(image), imageHash, 'Instrumented production addon changed')
} finally {
  writeFileSync(
    join(evidence, 'run.json'),
    JSON.stringify(
      {
        node: process.execPath,
        nodeVersion: process.version,
        platform: process.platform,
        image,
        imageHash,
        entry,
        commands,
      },
      null,
      2,
    ) + '\n',
    { flag: 'wx' },
  )
  console.info(`Native coverage evidence: ${evidence}`)
}
