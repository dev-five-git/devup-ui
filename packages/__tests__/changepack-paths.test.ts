import assert from 'node:assert/strict'
import { resolve } from 'node:path'

import { describe, expect, it } from 'bun:test'

const root = resolve(import.meta.dir, '../..')
// Bun's filesystem API avoids the node:fs spies used by the plugin suites.
const config: unknown = await Bun.file(
  resolve(root, '.changepacks/config.json'),
).json()
assert(typeof config === 'object' && config !== null && 'ignore' in config)
assert(Array.isArray(config.ignore))

// This is deliberately scoped to the asserted config, not a gitignore parser.
const managedManifests = new Set(
  config.ignore.slice(1).flatMap((pattern: unknown) => {
    assert(typeof pattern === 'string')
    return [
      ...new Bun.Glob(pattern.slice(1)).scanSync({
        cwd: root,
        onlyFiles: true,
      }),
    ]
      .map((path) => path.replaceAll('\\', '/'))
      .filter((path) =>
        /^(packages|bindings)\/[^/]+\/package\.json$/.test(path),
      )
  }),
)

function unmanagedPaths(paths: readonly string[]): string[] {
  // Do not normalize log keys: aliases must not silently resolve as canonical keys.
  return paths.filter((path) => !managedManifests.has(path)).sort()
}

describe('changepack manifest paths', () => {
  it('uses the supported managed-package allowlist when reading current config', () => {
    expect(config.ignore).toEqual([
      '*',
      '!packages/*/*',
      '!bindings/*/package.json',
    ])
  })

  it.each([
    'packages/react/package.json',
    'bindings/devup-ui-wasm/package.json',
  ])('accepts %s when it is an existing managed manifest', (path) => {
    const rejected = unmanagedPaths([path])
    expect(rejected).toEqual([])
  })

  it.each([
    'apps/landing/package.json',
    'package.json',
    'benchmark/tailwindcss/package.json',
    'packages/__tests__/package.json',
    'bindings/__missing_changepack_manifest__/package.json',
    'packages/react/src/package.json',
    'packages/react/README.md',
    './packages/react/package.json',
    'packages//react/package.json',
    'packages/react/../react/package.json',
    'packages\\react\\package.json',
    'Packages/react/package.json',
    'packages/react/package.json/',
  ])('rejects %s when it is unmanaged, missing or noncanonical', (path) => {
    const rejected = unmanagedPaths([path])
    expect(rejected).toEqual([path])
  })

  it('resolves every changes key when reading all changepack logs', async () => {
    const logs = [
      ...new Bun.Glob('.changepacks/changepack_log_*.json').scanSync({
        cwd: root,
        onlyFiles: true,
      }),
    ]
      .map((path) => path.replaceAll('\\', '/'))
      .sort()
    const diagnostics: string[] = []
    for (const path of logs) {
      const log: unknown = await Bun.file(resolve(root, path)).json()
      assert(typeof log === 'object' && log !== null && 'changes' in log, path)
      assert(typeof log.changes === 'object' && log.changes !== null, path)
      assert(!Array.isArray(log.changes), path)
      diagnostics.push(
        ...unmanagedPaths(Object.keys(log.changes)).map(
          (key) => `${path}: ${key}`,
        ),
      )
    }
    expect(diagnostics.sort()).toEqual([])
  })
})
