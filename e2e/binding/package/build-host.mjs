import { execFileSync } from 'node:child_process'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { targets } from './matrix.mjs'

const root = fileURLToPath(new URL('../../../', import.meta.url))
const actualHost = {
  platform: process.platform,
  arch: process.arch,
  report: process.report.getReport,
}
const actualIo = {
  run: execFileSync,
  now: () => performance.now(),
  emit: (record) => console.info(JSON.stringify(record)),
}

export class HostBuildTargetError extends Error {
  constructor(host, cause) {
    super(
      `Unsupported or indeterminate native host: ${host.platform}/${host.arch}`,
      { cause },
    )
    this.name = 'HostBuildTargetError'
    this.platform = host.platform
    this.arch = host.arch
  }
}

export function selectHostTarget(host = actualHost) {
  const candidates = targets.filter(
    (row) => row.os === host.platform && row.cpu === host.arch,
  )
  if (candidates.length === 0) throw new HostBuildTargetError(host)
  if (host.platform !== 'linux') return candidates[0]
  try {
    const report = host.report()
    if (
      typeof report !== 'object' ||
      report === null ||
      typeof report.header !== 'object' ||
      report.header === null
    ) {
      throw new HostBuildTargetError(host)
    }
    const libc = report.header.glibcVersionRuntime ? 'glibc' : 'musl'
    return candidates.find((row) => row.libc === libc)
  } catch (cause) {
    if (cause instanceof HostBuildTargetError) throw cause
    throw new HostBuildTargetError(host, cause)
  }
}

export function buildHost(host = actualHost, io = actualIo) {
  const row = selectHostTarget(host)
  if (!row) throw new HostBuildTargetError(host)
  const context = {
    event: 'devup.host-native-build',
    host: `${host.platform}/${host.arch}`,
    target: row.target,
    suffix: row.suffix,
    node: process.version,
    job: process.env.GITHUB_JOB ?? null,
    run: process.env.GITHUB_RUN_ID ?? null,
  }
  const started = io.now()
  io.emit({ ...context, phase: 'start' })
  let status = 'failed'
  try {
    io.run(
      process.execPath,
      [
        resolve(root, 'bindings/devup-ui-binding/build.mjs'),
        '--target',
        row.target,
      ],
      {
        cwd: root,
        env: process.env,
        stdio: 'inherit',
      },
    )
    status = 'success'
  } finally {
    io.emit({
      ...context,
      phase: 'finish',
      status,
      elapsedMs: io.now() - started,
    })
  }
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  buildHost()
