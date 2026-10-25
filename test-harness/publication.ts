import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join, relative, resolve } from 'node:path'

import { auditCoverage } from './coverage'
import { CoverageError, parseLcov } from './lcov'
import {
  publicationSources,
  rootRequired,
  viteRequired,
} from './source-inventory'

export type NativeLane = 'root' | 'vite'

export function digest(bytes: string | Buffer): string {
  return createHash('sha256').update(bytes).digest('hex')
}

export function publicationIdentity(root: string): string {
  const files = publicationSources(root)
  return JSON.stringify({
    executable: process.execPath,
    executableHash: digest(readFileSync(process.execPath)),
    bun: Bun.version,
    revision: Bun.revision,
    files: files.map((file) => [file, digest(readFileSync(join(root, file)))]),
  })
}

export function beginPublication(root: string): void {
  const coverage = join(root, 'coverage')
  mkdirSync(coverage, { recursive: true })
  for (const path of [
    'root',
    'vite',
    'accepted.json',
    'root.sha256',
    'vite.sha256',
    'session.json',
    'root.pending',
  ])
    rmSync(join(coverage, path), { recursive: true, force: true })
  writeFileSync(join(coverage, 'session.json'), publicationIdentity(root), {
    flag: 'wx',
  })
}

export function auditLane(
  root: string,
  lane: NativeLane,
  required: readonly string[],
): string {
  const coverage = join(root, 'coverage')
  if (
    readFileSync(join(coverage, 'session.json'), 'utf8') !==
    publicationIdentity(root)
  )
    throw new CoverageError('source/config/command/runtime identity changed')
  const raw = readFileSync(join(coverage, lane, 'lcov.info'))
  const sources = parseLcov(raw.toString()).map((source) => ({
    ...source,
    file: relative(root, resolve(root, source.file)).replaceAll('\\', '/'),
  }))
  auditCoverage(sources, required)
  return digest(raw)
}

export function recordRoot(
  root: string,
  required: readonly string[] = rootRequired,
): void {
  const hash = auditLane(root, 'root', required)
  writeFileSync(join(root, 'coverage/root.sha256'), hash, { flag: 'wx' })
  writeFileSync(join(root, 'coverage/root.pending'), hash, { flag: 'wx' })
}

export function continuePublication(
  root: string,
  required: readonly string[] = rootRequired,
): void {
  const coverage = join(root, 'coverage')
  rmSync(join(coverage, 'accepted.json'), { force: true })
  const pending = join(coverage, 'root.pending')
  const hash = readFileSync(pending, 'utf8')
  rmSync(pending)
  if (
    auditLane(root, 'root', required) !== hash ||
    readFileSync(join(coverage, 'root.sha256'), 'utf8') !== hash
  )
    throw new CoverageError(
      'ROOT continuation no longer matches native success',
    )
  rmSync(join(coverage, 'vite'), { recursive: true, force: true })
  rmSync(join(coverage, 'vite.sha256'), { force: true })
}

export function recordVite(
  root: string,
  required: readonly string[] = viteRequired(root),
): void {
  const hash = auditLane(root, 'vite', required)
  writeFileSync(join(root, 'coverage/vite.sha256'), hash, { flag: 'wx' })
}

export function acceptPublication(
  root: string,
  required: {
    readonly root: readonly string[]
    readonly vite: readonly string[]
  } = { root: rootRequired, vite: viteRequired(root) },
): void {
  const coverage = join(root, 'coverage')
  rmSync(join(coverage, 'accepted.json'), { force: true })
  const rootHash = auditLane(root, 'root', required.root)
  if (readFileSync(join(coverage, 'root.sha256'), 'utf8') !== rootHash)
    throw new CoverageError('root report changed after native success')
  const viteHash = auditLane(root, 'vite', required.vite)
  if (readFileSync(join(coverage, 'vite.sha256'), 'utf8') !== viteHash)
    throw new CoverageError('vite report changed after native success')
  writeFileSync(
    join(coverage, 'accepted.json'),
    JSON.stringify({
      identity: digest(publicationIdentity(root)),
      reports: { root: rootHash, vite: viteHash },
    }),
    { flag: 'wx' },
  )
  rmSync(join(coverage, 'root.pending'), { force: true })
}
