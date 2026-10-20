import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  closeSync,
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readdirSync,
  readFileSync,
  realpathSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

import { audit } from './audit.mjs'

assert.equal(process.argv.length, 3, 'Usage: node run.mjs <evidence-directory>')
const scratch = dirname(fileURLToPath(import.meta.url))
const destination = resolve(process.argv[2])
const location = relative(scratch, destination)
assert.ok(
  location === '..' || location.startsWith(`..${sep}`) || isAbsolute(location),
  'Keep evidence outside scratch',
)
mkdirSync(destination, { recursive: true })
const output = realpathSync(mkdtempSync(join(destination, 'W74a-')))
const hash = (path) =>
  createHash('sha256').update(readFileSync(path)).digest('hex')
const save = (name, value) =>
  writeFileSync(join(output, name), `${JSON.stringify(value, null, 2)}\n`)
const inputs = (
  'Cargo.toml Cargo.lock core.rs native_host.rs host.cjs ' +
  'addon/Cargo.toml addon/build.rs addon/lib.rs run.mjs audit.mjs'
).split(' ')
const hashes = () =>
  Object.fromEntries(inputs.map((path) => [path, hash(join(scratch, path))]))
const original = hashes()
assert.ok(!process.env.RUST_MIN_STACK, 'Default Rust stack required')
const env = {
  ...process.env,
  CARGO_BUILD_JOBS: '2',
  RUSTUP_TOOLCHAIN: 'stable',
}
delete env.RUSTFLAGS
delete env.CARGO_ENCODED_RUSTFLAGS
delete env.LLVM_PROFILE_FILE
const evidence = {
  verdict: 'INCOMPLETE',
  platform: process.platform,
  arch: process.arch,
  node: process.version,
  sources: original,
  controls: [],
  limits: [
    'SPIKE only, not production coverage.',
    'build.rs/native_host.rs 0/0 entries are unmeasured scaffolding, not measured coverage.',
    'Hit counts are used only as zero/nonzero; raw JSON/XML are retained.',
  ],
}
function run(name, command, childEnv = env) {
  const [program, ...args] = command
  const stdout = join(output, `${name}.stdout.log`)
  const stderr = join(output, `${name}.stderr.log`)
  const handles = [openSync(stdout, 'wx'), openSync(stderr, 'wx')]
  let result
  try {
    result = spawnSync(program, args, {
      cwd: scratch,
      env: childEnv,
      stdio: ['ignore', ...handles],
    })
  } finally {
    handles.forEach((handle) => closeSync(handle))
  }
  save(`${name}.command.json`, {
    command,
    cwd: scratch,
    status: result.status,
    signal: result.signal,
    rustflags: childEnv.RUSTFLAGS ?? null,
    llvmProfileFile: childEnv.LLVM_PROFILE_FILE ?? null,
    error: result.error?.message,
  })
  if (result.error) throw result.error
  assert.equal(result.signal, null, `${name}: natural child exit required`)
  return {
    status: result.status,
    stdout: readFileSync(stdout, 'utf8'),
    stderr: readFileSync(stderr, 'utf8'),
  }
}
try {
  for (const [name, command] of [
    ['rust', ['rustc', '+stable', '-Vv']],
    ['cargo', ['cargo', '+stable', '-V']],
    ['tarpaulin', ['cargo', '+stable', 'tarpaulin', '--version']],
    ['node', [process.execPath, '--version']],
    ['bun', ['bun', '--version']],
  ]) {
    const version = run(name, command)
    assert.equal(version.status, 0, `${name}: version check failed`)
    if (name === 'bun') evidence.bun = version.stdout.trim()
  }
  const bun = run('bun-path', [
    'bun',
    '-p',
    'JSON.stringify({execPath:process.execPath,version:process.versions.bun})',
  ])
  assert.equal(bun.status, 0)
  const bunRuntime = JSON.parse(bun.stdout)
  assert.equal(
    bunRuntime.version,
    evidence.bun,
    'Bun runtime identity mismatch',
  )
  assert.ok(isAbsolute(bunRuntime.execPath), 'Absolute Bun executable required')
  const target = join(output, 'build-target')
  const build = run(
    'build',
    [
      'cargo',
      '+stable',
      ...'build --locked -p w74a-addon --jobs 2'.split(' '),
      '--target-dir',
      target,
    ],
    {
      ...env,
      RUSTFLAGS: '-C instrument-coverage -C debuginfo=2 -C strip=none',
      LLVM_PROFILE_FILE: join(output, 'build_%m_%p.profraw'),
    },
  )
  assert.equal(build.status, 0, 'Instrumented debug build failed')
  const library = {
    win32: 'w74a_addon.dll',
    linux: 'libw74a_addon.so',
    darwin: 'libw74a_addon.dylib',
  }[process.platform]
  assert.ok(library, 'Unsupported scratch host')
  const built = join(target, 'debug', library)
  const image = join(output, 'W74a-addon.node')
  copyFileSync(built, image)
  evidence.image = {
    path: image,
    sha256: hash(image),
    built,
    builtSha256: hash(built),
  }
  assert.equal(evidence.image.sha256, evidence.image.builtSha256)
  let denominator
  for (const name of ['negative', 'positive']) {
    const negative = name === 'negative'
    const controlTarget = join(output, `${name}-target`)
    const reports = join(output, name)
    mkdirSync(reports)
    assert.deepEqual(hashes(), original, 'Inputs changed between controls')
    const result = run(
      name,
      [
        'cargo',
        '+stable',
        ...(
          'tarpaulin --locked --workspace --test W74a_native_host --engine llvm ' +
          '--skip-clean --jobs 2 --out Json --out Xml --out Stdout --fail-under 100'
        ).split(' '),
        '--target-dir',
        controlTarget,
        '--objects',
        image,
        '--output-dir',
        reports,
      ],
      {
        ...env,
        W74A_ADDON: image,
        W74A_CONTROL: negative ? 'negative' : 'positive',
        W74A_HOST: process.execPath,
      },
    )
    const profileRoot = join(controlTarget, 'tarpaulin', 'profraws')
    const profiles = readdirSync(profileRoot)
      .filter((path) => path.endsWith('.profraw'))
      .map((path) => {
        const fullPath = join(profileRoot, path)
        return {
          path: fullPath,
          bytes: statSync(fullPath).size,
          sha256: hash(fullPath),
        }
      })
    const entry = {
      name,
      status: result.status,
      target: controlTarget,
      reports,
      profiles,
      image,
      sourceHashes: hashes(),
      imageSha256: hash(image),
    }
    evidence.controls.push(entry)
    save(`${name}.profiles.json`, profiles)
    entry.reportHashes = {
      json: hash(join(reports, 'tarpaulin-report.json')),
      xml: hash(join(reports, 'cobertura.xml')),
    }
    denominator = audit(entry, result, denominator)
    assert.deepEqual(hashes(), original)
    assert.deepEqual(entry.sourceHashes, original)
    assert.equal(entry.imageSha256, evidence.image.sha256)
    assert.equal(hash(image), evidence.image.sha256)
    save(`${name}.inventory.json`, entry)
  }
  assert.deepEqual(hashes(), original, 'Inputs changed before Bun execution')
  assert.equal(hash(image), evidence.image.sha256)
  const bunResult = run(
    'bun-positive',
    [bunRuntime.execPath, join(scratch, 'host.cjs')],
    {
      ...env,
      W74A_CONTROL: 'positive',
      W74A_ADDON: image,
      LLVM_PROFILE_FILE: join(output, 'bun-execution_%m_%p.profraw'),
    },
  )
  assert.equal(
    bunResult.status,
    0,
    'Bun: positive host must exit naturally with 0',
  )
  const bunHost = JSON.parse(
    bunResult.stdout.match(/\{"event":"W74a.host"[^\n]*\}/)?.[0] ?? 'null',
  )
  assert.equal(bunHost.requestedAddon, image)
  assert.equal(bunHost.loadedAddon, image)
  assert.equal(bunHost.control, 'positive')
  evidence.bunExecution = {
    kind: 'real-addon execution only; not Rust coverage',
    runtime: bunRuntime,
    status: bunResult.status,
    host: bunHost,
    sourceHashes: hashes(),
    imageSha256: hash(image),
    builtSha256: hash(built),
  }
  assert.deepEqual(evidence.bunExecution.sourceHashes, original)
  assert.equal(evidence.bunExecution.imageSha256, evidence.image.sha256)
  assert.equal(evidence.bunExecution.builtSha256, evidence.image.builtSha256)
  evidence.verdict = 'SCRATCH CONTROLS PASS; production coverage unproved'
} finally {
  save('evidence.json', evidence)
  console.info(`W74a evidence: ${output}`)
}
