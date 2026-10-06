import * as fs from 'node:fs'
import { dirname, join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { buildStaticImportGraph, createModuleResolver } from '../index'
import {
  createResolutionInputs,
  readResolutionFile,
  realpathResolution,
} from '../resolution-inputs'
import { resolutionWatchPath } from '../resolution-watch-path'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it('records consulted manifests and missing probes when resolving a linked package', () => {
  // Given a linked workspace package with an exports-only entry.
  file('src/main.ts', "import 'palette'")
  const manifest = file(
    'packages/palette/package.json',
    '{"exports":"./red.js"}',
  )
  file('packages/palette/red.js', "export const color='red'")
  file('node_modules/placeholder/package.json', '{}')
  fs.symlinkSync(
    dirname(manifest),
    join(root, 'node_modules/palette'),
    'junction',
  )
  const originalRead = fs.readFileSync
  const reads: string[] = []
  const read = spyOn(fs, 'readFileSync').mockImplementation(
    new Proxy(originalRead, {
      apply(target, receiver, args) {
        reads.push(String(args[0]))
        return Reflect.apply(target, receiver, args)
      },
    }),
  )
  try {
    // When scanning with real filesystem IO.
    const graph = buildStaticImportGraph('src', undefined, { cwd: root })
    // Then the outcome names the lexical consulted manifest, not a guessed physical read.
    const lexical = join(root, 'node_modules/palette/package.json')
    expect(reads.filter((path) => path.endsWith('package.json'))).toEqual([
      lexical,
    ])
    expect(graph.requests[0]?.outcome).toMatchObject({
      kind: 'resolved',
      inputs: {
        fileDependencies: [
          lexical,
          join(root, 'node_modules/palette/red.js'),
        ].sort(),
        missingDependencies: [
          join(root, 'src/node_modules/palette/package.json'),
          join(root, 'tsconfig.json'),
        ].sort(),
      },
    })
  } finally {
    read.mockRestore()
  }
})

it('observes setup inheritance and failed earlier candidates when evaluating an import', () => {
  // Given inherited aliases with eager multiple targets.
  const config = file(
    'tsconfig.json',
    '{"extends":"./base","compilerOptions":{}}',
  )
  const base = file(
    'base.json',
    '{"compilerOptions":{"paths":{"color":["absent","red.ts","later"]}}}',
  )
  const red = file('red.ts', "export const color='red'")
  const observations: unknown[] = []
  // When the public callable resolves and reads source.
  const resolver = createModuleResolver({
    cwd: root,
    onResolutionInputs: (inputs) => observations.push(inputs),
  })
  expect(resolver('color', join(root, 'main.ts'))).toMatchObject({ path: red })
  // Then actual setup and eager failed candidates are reported without changing resolution.
  expect(observations).toEqual(
    expect.arrayContaining([
      expect.objectContaining({
        fileDependencies: [base, config].sort(),
        missingDependencies: [join(root, 'base')],
      }),
      expect.objectContaining({
        fileDependencies: [base, red, config].sort(),
        missingDependencies: expect.arrayContaining([
          join(root, 'absent'),
          join(root, 'later'),
        ]),
      }),
    ]),
  )
  expect(observations.every((inputs) => Object.isFrozen(inputs))).toBe(true)
})

it('freezes deterministic snapshots without redoing successful resolution IO', () => {
  // Given a real extensionless import and unchanged filesystem.
  file('src/main.ts', "import './red'")
  file('src/red.ts', "export const color='red'")
  // When scanning twice independently.
  const first = buildStaticImportGraph('src', undefined, { cwd: root })
  const second = buildStaticImportGraph('src', undefined, { cwd: root })
  // Then the full evidence is stable and its nested arrays cannot change.
  expect(first.requests).toEqual(second.requests)
  const outcome = first.requests[0]?.outcome
  expect(outcome?.kind).toBe('resolved')
  if (outcome && 'inputs' in outcome) {
    expect(Object.isFrozen(outcome.inputs)).toBe(true)
    expect(Object.isFrozen(outcome.inputs.fileDependencies)).toBe(true)
    expect(Object.isFrozen(outcome.inputs.missingDependencies)).toBe(true)
  }
})

it.each(['main', 'fallback', 'json'])(
  'records actual loaded packaged config chain when using %s resolution',
  (mode) => {
    // Given a package-config extension found below an absent nearer node_modules directory.
    const config = file('src/tsconfig.json', '{"extends":"preset"}')
    const manifest = file(
      'node_modules/preset/package.json',
      mode === 'main'
        ? '{"main":"index.js","tsconfig":"base.json"}'
        : mode === 'json'
          ? '{"main":"base.json"}'
          : '{}',
    )
    if (mode === 'main')
      file('node_modules/preset/index.js', 'module.exports={}')
    const base = file(
      `node_modules/preset/${mode === 'fallback' ? 'tsconfig.json' : 'base.json'}`,
      '{}',
    )
    const observations: unknown[] = []
    // When constructing the existing synchronous resolver.
    createModuleResolver({
      cwd: root,
      tsconfigPath: config,
      onResolutionInputs: (inputs) => observations.push(inputs),
    })
    // Then only owned reads and public-search missing directories are recorded.
    expect(observations).toContainEqual({
      fileDependencies: (mode === 'main'
        ? [config, manifest, base]
        : [config, base]
      ).sort(),
      missingDependencies: [join(root, 'src/node_modules/preset')],
    })
  },
)

it('reports a missing inherited configuration before preserving its established failure', () => {
  // Given a top config pointing at a nonexistent parent.
  file('tsconfig.json', '{"extends":"./missing"}')
  const observations: unknown[] = []
  // When setup reaches the original IO failure.
  expect(() =>
    createModuleResolver({
      cwd: root,
      onResolutionInputs: (inputs) => observations.push(inputs),
    }),
  ).toThrow(join(root, 'missing.json'))
  // Then the repair paths were observed even though no callable was returned.
  expect(observations).toContainEqual({
    fileDependencies: [join(root, 'tsconfig.json')],
    missingDependencies: [
      join(root, 'missing'),
      join(root, 'missing.json'),
    ].sort(),
  })
})

it('canonicalizes only transport identity when watching a missing descendant below a symlink', () => {
  // Given a symlink and a missing file below its existing target.
  const physical = file('physical/existing.ts')
  file('links/placeholder.ts')
  fs.symlinkSync(dirname(physical), join(root, 'links/package'), 'junction')
  const lexical = join(root, 'links/package/new.ts')
  // When selecting default and preserved transport identities.
  const canonical = resolutionWatchPath(lexical)
  // Then resolver evidence remains lexical while transport follows the bundler setting.
  expect(canonical).toBe(join(root, 'physical/new.ts'))
  expect(resolutionWatchPath(lexical, true)).toBe(lexical)
})

it('preserves IO failure identity and records attempted repair inputs', () => {
  // Given a permission failure at the narrow existing read seam.
  const target = file('blocked.json')
  const original = Object.assign(new Error('denied'), { code: 'EACCES' })
  const originalRead = fs.readFileSync
  const read = spyOn(fs, 'readFileSync').mockImplementation(
    new Proxy(originalRead, {
      apply(fn, receiver, args) {
        if (args[0] === target) throw original
        return Reflect.apply(fn, receiver, args)
      },
    }),
  )
  const inputs = createResolutionInputs()
  try {
    // When reading the input through the observed seam.
    let caught: unknown
    try {
      readResolutionFile(target, inputs)
    } catch (error) {
      caught = error
    }
    // Then the original exception and actual attempted filename survive.
    expect(caught).toBe(original)
    expect(inputs.snapshot()).toEqual({
      fileDependencies: [target],
      missingDependencies: [],
    })
  } finally {
    read.mockRestore()
  }
})

it.each(['EACCES', 'ENOENT'])(
  'retains lexical watch identity when canonicalization fails with %s',
  (code) => {
    // Given a transport-only realpath failure, including an unavailable root.
    const path = join(root, 'unavailable.ts')
    const original = Object.assign(new Error('unavailable'), { code })
    const realpath = spyOn(fs, 'realpathSync').mockImplementation(() => {
      throw original
    })
    try {
      // When asking for watcher identity without changing resolution semantics.
      const watched = resolutionWatchPath(path)
      // Then no secondary error replaces the original extraction/config failure.
      expect(watched).toBe(path)
    } finally {
      realpath.mockRestore()
    }
  },
)

it('records a failed canonical probe without replacing the original IO error', () => {
  // Given a real file whose existing canonicalization seam fails after resolution probes.
  const path = file('canonical.ts')
  const original = Object.freeze(
    Object.assign(new Error('removed during resolution'), { code: 'ENOENT' }),
  )
  const realpath = spyOn(fs, 'realpathSync').mockImplementation(() => {
    throw original
  })
  const inputs = createResolutionInputs()
  try {
    // When observing that existing realpath invocation.
    let caught: unknown
    try {
      realpathResolution(path, inputs)
    } catch (error) {
      caught = error
    }
    // Then its exact path is repairable and its thrown object is identical.
    expect(caught).toBe(original)
    expect(inputs.snapshot()).toEqual({
      fileDependencies: [],
      missingDependencies: [path],
    })
  } finally {
    realpath.mockRestore()
  }
})
