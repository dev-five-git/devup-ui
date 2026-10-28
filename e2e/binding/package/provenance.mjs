import assert from 'node:assert/strict'
import { execFileSync, spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { dirname, isAbsolute, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

import { inspectImage, targets } from './matrix.mjs'

export const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..')
export const sha256 = (bytes) =>
  createHash('sha256').update(bytes).digest('hex')
export const hashFile = (path) => sha256(readFileSync(path))
export const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'))
export const writeJson = (path, value) =>
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`)
export const direct = (url) =>
  process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === url
export function command(program, args, options = {}) {
  const output = execFileSync(program, args, {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
    ...options,
  })
  return typeof output === 'string' ? output.trim() : output
}
export function sourceIdentity() {
  const vcs = (args) =>
    command('git', args, { env: { ...process.env, GIT_MASTER: '1' } })
  const paths = vcs([
    'ls-files',
    '--cached',
    '--others',
    '--exclude-standard',
    '-z',
  ])
    .split('\0')
    .filter(Boolean)
    .sort()
  const files = Object.fromEntries(
    paths.map((path) => [
      path,
      existsSync(join(root, path)) ? hashFile(join(root, path)) : null,
    ]),
  )
  return {
    commit: vcs(['rev-parse', 'HEAD']),
    tree: vcs(['rev-parse', 'HEAD^{tree}']),
    dirty: Boolean(vcs(['status', '--porcelain'])),
    digest: sha256(JSON.stringify(files)),
    files,
  }
}
export function tool(program, args = ['--version']) {
  const executable = isAbsolute(program)
    ? program
    : ['rustc', 'cargo'].includes(program)
      ? command('rustup', ['which', program])
      : process.platform === 'win32'
        ? command('where.exe', [program]).split(/\r?\n/)[0]
        : command('which', [program])
  return {
    executable,
    sha256: hashFile(executable),
    version: command(executable, args),
  }
}

export function buildArtifact(options) {
  const row = targets.find((item) => item.target === options.target)
  assert.ok(row, 'unknown target')
  assert.ok(options.run, 'explicit same-run identity required')
  const source = sourceIdentity()
  assert.ok(
    options.allowDirty || !source.dirty,
    'CI requires clean committed source',
  )
  mkdirSync(options.out, { recursive: false })
  const receiptPath = join(options.out, 'receipt.json')
  const receipt = {
    schema: 1,
    status: 'building',
    run: options.run,
    source,
    row,
    profile: 'native',
    started: new Date().toISOString(),
  }
  writeJson(receiptPath, receipt)
  try {
    const args = [
      'bindings/devup-ui-binding/build.mjs',
      '--target',
      row.target,
      ...(options.zigbuild ? ['--zigbuild'] : []),
    ]
    receipt.command = [process.execPath, ...args]
    receipt.cargoCommand = [
      'cargo',
      options.zigbuild ? 'zigbuild' : 'build',
      '--locked',
      '--profile',
      'native',
      '--target',
      options.zigbuild && row.libc === 'glibc'
        ? `${row.target}.2.28`
        : row.target,
      '--jobs',
      '2',
      '-p',
      'devup-ui-binding',
    ]
    receipt.environment = Object.fromEntries(
      [
        'CARGO_BUILD_JOBS',
        'CARGO_TARGET_DIR',
        'RUSTUP_TOOLCHAIN',
        'RUSTFLAGS',
        'RUST_MIN_STACK',
        'CI',
        'PATH',
        'SDKROOT',
        'MACOSX_DEPLOYMENT_TARGET',
        'INCLUDE',
        'LIB',
      ].map((key) => [key, process.env[key] ?? null]),
    )
    receipt.identity = Object.fromEntries(
      [
        'GITHUB_SHA',
        'GITHUB_HEAD_REF',
        'GITHUB_BASE_REF',
        'GITHUB_RUN_ID',
        'GITHUB_RUN_ATTEMPT',
        'RUNNER_OS',
        'RUNNER_ARCH',
        'ImageOS',
        'ImageVersion',
      ].map((key) => [key, process.env[key] ?? null]),
    )
    const event = process.env.GITHUB_EVENT_PATH
      ? readJson(process.env.GITHUB_EVENT_PATH)
      : undefined
    receipt.pullRequest = {
      head: event?.pull_request?.head?.sha ?? null,
      base: event?.pull_request?.base?.sha ?? null,
      merge: event?.pull_request?.merge_commit_sha ?? null,
    }
    receipt.runner = { platform: process.platform, arch: process.arch }
    receipt.tools = {
      node: tool(process.execPath),
      bun: tool('bun'),
      cargo: tool('cargo'),
      rustc: tool('rustc', ['-Vv']),
      ...(options.zigbuild
        ? {
            zig: tool('zig', ['version']),
            zigbuild: tool('cargo-zigbuild'),
          }
        : {}),
    }
    const build = spawnSync(process.execPath, args, {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 32 * 1024 * 1024,
    })
    writeFileSync(join(options.out, 'build.stdout'), build.stdout || '')
    writeFileSync(join(options.out, 'build.stderr'), build.stderr || '')
    assert.equal(
      build.status,
      0,
      build.error?.message || 'native build failed; see retained logs',
    )
    const library = {
      win32: 'devup_ui_binding.dll',
      linux: 'libdevup_ui_binding.so',
      darwin: 'libdevup_ui_binding.dylib',
    }[row.os]
    const original = join(
      resolve(root, process.env.CARGO_TARGET_DIR || 'target'),
      row.target,
      'native',
      library,
    )
    const image = `binding.${row.suffix}.node`
    const attached = join(options.out, image)
    copyFileSync(
      join(root, `bindings/devup-ui-binding-${row.suffix}`, image),
      attached,
    )
    assert.ok(
      statSync(original).mtimeMs >= Date.parse(receipt.started) - 2000,
      'stale build output',
    )
    receipt.original = { path: original, sha256: hashFile(original) }
    receipt.image = {
      file: image,
      sha256: hashFile(attached),
      bytes: statSync(attached).size,
      evidence: inspectImage(attached, row),
    }
    assert.equal(receipt.original.sha256, receipt.image.sha256)
    assert.deepEqual(sourceIdentity(), source, 'source changed during build')
    receipt.status = 'built'
    receipt.finished = new Date().toISOString()
    return receipt
  } catch (error) {
    receipt.status = 'failed'
    receipt.error = { message: error.message, stack: error.stack }
    if (error.stdout)
      writeFileSync(join(options.out, 'build.stdout'), error.stdout)
    if (error.stderr)
      writeFileSync(join(options.out, 'build.stderr'), error.stderr)
    throw error
  } finally {
    writeJson(receiptPath, receipt)
  }
}
if (direct(import.meta.url)) {
  const [target, out, run, ...flags] = process.argv.slice(2)
  assert.ok(
    flags.every((flag) => ['--allow-dirty', '--zigbuild'].includes(flag)),
  )
  buildArtifact({
    target,
    out,
    run,
    allowDirty: flags.includes('--allow-dirty'),
    zigbuild: flags.includes('--zigbuild'),
  })
}
