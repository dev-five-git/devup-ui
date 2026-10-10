import { createHash } from 'node:crypto'
import * as fs from 'node:fs'
import { dirname, join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { resolvePackage } from '../owned-module-resolution'
import { createResolutionInputs } from '../resolution-inputs'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it('reads the matched root manifest when a nested config has an unrelated manifest', () => {
  // Given a native public lookup resolving below an absent nearer package directory.
  const importer = file('nested/tsconfig.json', '{}')
  const manifest = file('node_modules/preset/package.json', '{}')
  file('node_modules/preset/configs/package.json', '{}')
  file('node_modules/preset/configs/base.json', '{}')
  const inputs = createResolutionInputs()
  const reads: string[] = []
  const originalRead = fs.readFileSync
  const read = spyOn(fs, 'readFileSync').mockImplementation(
    new Proxy(originalRead, {
      apply(target, receiver, args) {
        reads.push(String(args[0]))
        return Reflect.apply(target, receiver, args)
      },
    }),
  )
  // When recording public search-directory checks for the completed native lookup.
  try {
    expect(
      resolvePackage('preset/configs/base.json', importer, {
        purpose: 'tsconfig-extends',
        inputs,
      }),
    ).toBe(join(root, 'node_modules/preset/configs/base.json'))
    // Then the exact owned read excludes the nearer nested manifest and later search directories.
    expect(reads).toEqual([manifest])
    expect(inputs.snapshot().fileDependencies).toEqual(
      [manifest, join(root, 'node_modules/preset/configs/base.json')].sort(),
    )
    expect(inputs.snapshot().missingDependencies).toContain(
      join(root, 'nested/node_modules/preset/package.json'),
    )
  } finally {
    read.mockRestore()
  }
})

it('records the lexical selecting manifest when a scoped package is linked', () => {
  // Given a scoped package whose physical config lies outside node_modules.
  const importer = file('nested/tsconfig.json', '{}')
  const physical = file(
    'workspace/preset/package.json',
    '{"exports":"./base.json"}',
  )
  file('workspace/preset/base.json', '{}')
  file('node_modules/@scope/placeholder.json', '{}')
  fs.symlinkSync(
    dirname(physical),
    join(root, 'node_modules/@scope/preset'),
    'junction',
  )
  const inputs = createResolutionInputs()
  // When observing ownership using the already selected canonical path.
  expect(
    resolvePackage('@scope/preset', importer, {
      purpose: 'tsconfig-extends',
      inputs,
    }),
  ).toBe(join(root, 'workspace/preset/base.json'))
  // Then evidence retains the matched lexical root rather than its physical location.
  expect(inputs.snapshot().fileDependencies).toEqual(
    [
      join(root, 'node_modules/@scope/preset/package.json'),
      join(root, 'node_modules/@scope/preset/base.json'),
    ].sort(),
  )
  expect(inputs.snapshot().missingDependencies).toContain(
    join(root, 'nested/node_modules/@scope/preset/package.json'),
  )
})

it('records a missing exact manifest when a manifestless deep file resolves', () => {
  // Given a valid deep-file package without a root manifest.
  const importer = file('nested/tsconfig.json', '{}')
  file('node_modules/preset/base.json', '{}')
  const inputs = createResolutionInputs()
  // When observing the completed lookup without requiring a manifest to exist.
  expect(
    resolvePackage('preset/base.json', importer, {
      purpose: 'tsconfig-extends',
      inputs,
    }),
  ).toBe(join(root, 'node_modules/preset/base.json'))
  // Then the absent root manifest is repairable without rejecting the valid config.
  expect(inputs.snapshot().fileDependencies).toEqual([
    join(root, 'node_modules/preset/base.json'),
  ])
  expect(inputs.snapshot().missingDependencies).toContain(
    join(root, 'node_modules/preset/package.json'),
  )
})

it('skips an existing nearer package when it does not contain the selected file', () => {
  // Given a nearer package missing the requested deep file and a matching outer package.
  const importer = file('nested/tsconfig.json', '{}')
  const nearer = file('nested/node_modules/preset/package.json', '{}')
  const manifest = file('node_modules/preset/package.json', '{}')
  file('node_modules/preset/..configs/base.json', '{}')
  const inputs = createResolutionInputs()
  // When recording the native selected file rather than guessing the nearest package.
  expect(
    resolvePackage('preset/..configs/base.json', importer, {
      purpose: 'tsconfig-extends',
      inputs,
    }),
  ).toBe(join(root, 'node_modules/preset/..configs/base.json'))
  // Then containment recognizes a dot-prefixed child and excludes the unrelated manifest.
  expect(inputs.snapshot().fileDependencies).toEqual(
    [
      nearer,
      manifest,
      join(root, 'node_modules/preset/..configs/base.json'),
    ].sort(),
  )
  expect(inputs.snapshot().missingDependencies).toContain(
    join(root, 'nested/node_modules/preset/..configs/base.json'),
  )
})

it('preserves the real read failure when the matched manifest cannot be read', () => {
  // Given a resolved config and a failure at the owned manifest read boundary.
  const importer = file('tsconfig.json', '{}')
  const manifest = file('node_modules/preset/package.json', '{}')
  file('node_modules/preset/base.json', '{}')
  const failure = Object.assign(new Error('denied'), {
    code: 'EACCES',
    path: manifest,
  })
  const original = fs.readFileSync
  const read = spyOn(fs, 'readFileSync').mockImplementation(
    new Proxy(original, {
      apply(target, receiver, args) {
        if (args[0] === manifest) throw failure
        return Reflect.apply(target, receiver, args)
      },
    }),
  )
  const inputs = createResolutionInputs()
  try {
    // When the exact matched manifest read fails.
    let caught: unknown
    try {
      resolvePackage('preset', importer, {
        purpose: 'tsconfig-extends',
        inputs,
      })
    } catch (error) {
      caught = error
    }
    // Then neither the exception nor its attempted input identity is replaced.
    expect(caught).toBe(failure)
    expect(inputs.snapshot().fileDependencies).toEqual([manifest])
  } finally {
    read.mockRestore()
  }
})

it('changes the observed content fingerprint when only selecting exports changes', () => {
  // Given two preexisting configs and an observed selection, independent of native freshness.
  const importer = file('tsconfig.json', '{}')
  const manifest = file(
    'node_modules/preset/package.json',
    '{"exports":"./red.json"}',
  )
  file(
    'node_modules/preset/red.json',
    '{"compilerOptions":{"paths":{"color":["red.ts"]}}}',
  )
  file(
    'node_modules/preset/blue.json',
    '{"compilerOptions":{"paths":{"color":["blue.ts"]}}}',
  )
  const inputs = createResolutionInputs()
  resolvePackage('preset', importer, { purpose: 'tsconfig-extends', inputs })
  const before = createHash('sha256')
  for (const path of inputs.snapshot().fileDependencies)
    before.update(fs.readFileSync(path))
  // When only the selecting manifest content changes.
  fs.writeFileSync(manifest, '{"exports":"./blue.json"}')
  const after = createHash('sha256')
  for (const path of inputs.snapshot().fileDependencies)
    after.update(fs.readFileSync(path))
  // Then unchanged input paths still expose the selecting-content change.
  expect(after.digest('hex')).not.toBe(before.digest('hex'))
})
