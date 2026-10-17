import * as fs from 'node:fs'
import { join, resolve } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { createModuleResolver } from '../import-graph'
import { ConfigLoadError } from '../load-config'
import { createResolutionInputs } from '../resolution-inputs'
import { readPathAliases } from '../tsconfig'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it('records every actual failed lexical probe and consulted read without evidence-only lookups', () => {
  // Given a nearer miss, an exports array miss, and an outer resolving package.
  const config = file('src/tsconfig.json', '{"extends":"preset"}')
  const nearer = file('src/node_modules/preset/package.json', '{"exports":{}}')
  const manifest = file(
    'node_modules/preset/package.json',
    '{"exports":["./absent.json","./blue.json"]}',
  )
  const blue = file('node_modules/preset/blue.json', '{}')
  const missing = new Set<string>()
  const reads: string[] = []
  const originalStat = fs.statSync
  const originalRead = fs.readFileSync
  const stat = spyOn(fs, 'statSync').mockImplementation(
    new Proxy(originalStat, {
      apply(target, receiver, args) {
        try {
          return Reflect.apply(target, receiver, args)
        } catch (error) {
          if (
            error instanceof Error &&
            'code' in error &&
            (error.code === 'ENOENT' || error.code === 'ENOTDIR')
          )
            missing.add(resolve(String(args[0])))
          throw error
        }
      },
    }),
  )
  const read = spyOn(fs, 'readFileSync').mockImplementation(
    new Proxy(originalRead, {
      apply(target, receiver, args) {
        reads.push(String(args[0]))
        return Reflect.apply(target, receiver, args)
      },
    }),
  )
  const inputs = createResolutionInputs()
  try {
    // When one setup performs real owned selection.
    readPathAliases(config, inputs)
    // Then repair evidence is exactly the real lexical IO, with no second resolution pass.
    expect(reads).toEqual([config, nearer, manifest, blue])
    expect(inputs.snapshot()).toEqual({
      fileDependencies: [config, nearer, manifest, blue].sort(),
      missingDependencies: [...missing].sort(),
    })
    expect(missing).toContain(join(root, 'node_modules/preset/absent.json'))
  } finally {
    stat.mockRestore()
    read.mockRestore()
  }
})

it.each(['EACCES', 'EIO'])(
  'retains actual manifest read failure %s and its lexical repair input',
  (code) => {
    // Given a real selecting manifest whose read fails at the narrow filesystem seam.
    const config = file('tsconfig.json', '{"extends":"preset"}')
    const manifest = file(
      'node_modules/preset/package.json',
      '{"exports":"./blue.json"}',
    )
    file('node_modules/preset/blue.json', '{}')
    const failure = Object.assign(new Error('denied'), { code })
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
      let caught: unknown
      // When setup attempts the owned read.
      try {
        readPathAliases(config, inputs)
      } catch (error) {
        caught = error
      }
      // Then located config errors preserve the original IO exception, not a resolution miss.
      expect(caught).toBeInstanceOf(ConfigLoadError)
      if (caught instanceof ConfigLoadError) expect(caught.cause).toBe(failure)
      expect(inputs.snapshot().fileDependencies).toEqual(
        [config, manifest].sort(),
      )
    } finally {
      read.mockRestore()
    }
  },
)

it.each(['./exact.txt', './folder', './missing.json'])(
  'retains TypeScript direct extends rules for %s',
  (request) => {
    // Given an exact non-JSON config and a directory that must not gain index lookup.
    const config = file('tsconfig.json', JSON.stringify({ extends: request }))
    file('exact.txt', '{"compilerOptions":{"paths":{"value":["blue.ts"]}}}')
    file('folder/tsconfig.json', '{}')
    const inputs = createResolutionInputs()
    // When loading the direct extends branch.
    if (request === './exact.txt') {
      // Then existing exact filenames are accepted regardless of extension.
      expect(readPathAliases(config, inputs).aliases[0]?.targets).toEqual([
        join(root, 'blue.ts'),
      ])
    } else {
      expect(() => readPathAliases(config, inputs)).toThrow(
        'Cannot load configuration',
      )
      expect(inputs.snapshot().missingDependencies).toContain(
        join(root, request === './folder' ? 'folder.json' : 'missing.json'),
      )
    }
  },
)

it('publishes setup failure probes through the unchanged public observer', () => {
  // Given a blocked export with an existing unexported config.
  const config = file('tsconfig.json', '{"extends":"preset"}')
  const manifest = file(
    'node_modules/preset/package.json',
    '{"exports":{".":null}}',
  )
  file('node_modules/preset/tsconfig.json', '{}')
  const observations: unknown[] = []
  // When synchronous public setup fails.
  expect(() =>
    createModuleResolver({
      cwd: root,
      onResolutionInputs: (inputs) => observations.push(inputs),
    }),
  ).toThrow(`Cannot resolve tsconfig extends "preset"`)
  // Then its finally observer retains the real selecting manifest, even with no returned callable.
  expect(observations).toContainEqual(
    expect.objectContaining({ fileDependencies: [config, manifest].sort() }),
  )
})
