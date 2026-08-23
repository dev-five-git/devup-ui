import { existsSync } from 'node:fs'
import { readdir, readFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

// A bundler that drops a module but keeps its re-export clause produces a file
// that still *builds* (exit 0) and only fails when something loads it. Bun 1.4.0
// did exactly that for a barrel entry whose package declared
// `sideEffects: false`, shrinking `dist/index.mjs` to a 22-byte stub. Nothing in
// `bun run build` catches it, so a broken package can reach npm. Loading every
// declared entry and touching every export turns that silence into a failure.

// Thrown by devup-ui's compile-only stubs. A package like @devup-ui/reset-css
// calls one at module scope, so raising it proves the entry executed real code
// rather than collapsing into a stub — which is exactly what this check asks.
const COMPILE_ONLY_SENTINEL = 'Cannot run on the runtime'

interface PackageManifest {
  name?: string
  main?: string
  module?: string
  exports?: unknown
  files?: string[]
}

function collectEntries(node: unknown, found: Set<string>): void {
  if (typeof node === 'string') {
    if (node.startsWith('./dist/') && /\.(c|m)?js$/.test(node)) found.add(node)
    return
  }
  if (Array.isArray(node)) {
    for (const child of node) collectEntries(child, found)
    return
  }
  if (node && typeof node === 'object') {
    for (const child of Object.values(node)) collectEntries(child, found)
  }
}

async function verifyPackage(pkgDir: string): Promise<string[]> {
  const manifestPath = join(pkgDir, 'package.json')
  if (!existsSync(manifestPath)) return []
  const manifest: PackageManifest = JSON.parse(
    await readFile(manifestPath, 'utf-8'),
  )
  if (!manifest.files?.includes('dist')) return []

  const entries = new Set<string>()
  collectEntries(manifest.main, entries)
  collectEntries(manifest.module, entries)
  collectEntries(manifest.exports, entries)

  const failures: string[] = []
  for (const entry of [...entries].sort()) {
    const entryPath = resolve(pkgDir, entry)
    const label = `${manifest.name} ${entry}`
    if (!existsSync(entryPath)) {
      failures.push(`${label}: declared but missing on disk`)
      continue
    }
    try {
      const loaded: Record<string, unknown> = await import(
        pathToFileURL(entryPath).href
      )
      const keys = Object.keys(loaded)
      if (keys.length === 0) {
        failures.push(`${label}: loaded with zero exports`)
        continue
      }
      // Broken CJS output keeps the export getters and only throws on access.
      for (const key of keys) void loaded[key]
    } catch (error) {
      const message = (error as Error).message
      if (!message.includes(COMPILE_ONLY_SENTINEL)) {
        failures.push(`${label}: ${message}`)
      }
    }
  }
  return failures
}

const packagesDir = resolve(import.meta.dir, 'packages')
const dirs = await readdir(packagesDir, { withFileTypes: true })
const failures = (
  await Promise.all(
    dirs
      .filter((d) => d.isDirectory())
      .map((d) => verifyPackage(join(packagesDir, d.name))),
  )
).flat()

if (failures.length > 0) {
  console.error(`verify-dist: ${failures.length} broken entry point(s)`)
  for (const failure of failures) console.error(`  - ${failure}`)
  process.exit(1)
}
console.info('verify-dist: all declared dist entry points load and export')
