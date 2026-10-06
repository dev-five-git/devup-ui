import * as fs from 'node:fs'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, sep } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { registerLoaderMissingDependencies } from '../loader-resolution-watch'

it('watches the nearest existing directory once when missing probes share ancestors', () => {
  // Given
  const root = mkdtempSync(join(tmpdir(), 'devup-next-missing-'))
  const outside = join(root, 'outside')
  mkdirSync(outside)
  writeFileSync(join(outside, 'file'), '')
  const paths = [
    join(outside, 'earlier.js'),
    join(outside, 'absent', 'index.ts'),
    join(outside, 'file', 'child.js'),
    join(outside, 'file', 'deep', 'nested', 'index.cjs'),
    join(root, 'another', 'entry.js'),
  ]
  const missing: string[] = []
  const contexts: string[] = []
  const original = fs.statSync
  const stat = spyOn(fs, 'statSync').mockImplementation(
    (...args: unknown[]) => {
      const result = Reflect.apply(original, fs, args)
      // Keep real stats; simulate only Windows' absent file-descendant result.
      if (
        process.platform === 'win32' &&
        result === undefined &&
        typeof args[0] === 'string' &&
        args[0].startsWith(join(outside, 'file') + sep)
      )
        throw Object.assign(new Error('not a directory'), { code: 'ENOTDIR' })
      return result
    },
  )
  try {
    // When
    registerLoaderMissingDependencies(
      {
        _compiler: undefined,
        addMissingDependency: (path) => missing.push(path),
        addContextDependency: (path) => contexts.push(path),
      },
      paths,
    )
    // Then
    expect(missing).toEqual(paths)
    expect(contexts).toEqual([outside, root])
  } finally {
    stat.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
})

it.each(['EACCES', 'EIO'])(
  'preserves %s instead of walking past a failed ancestor stat',
  (code) => {
    // Given
    const failure = Object.assign(new Error('ancestor stat failed'), { code })
    const contexts: string[] = []
    const stat = spyOn(fs, 'statSync').mockImplementation(() => {
      throw failure
    })
    try {
      // When / Then
      expect(() =>
        registerLoaderMissingDependencies(
          {
            _compiler: undefined,
            addMissingDependency: () => undefined,
            addContextDependency: (path) => contexts.push(path),
          },
          [join('outside', 'missing', 'index.cjs')],
        ),
      ).toThrow(failure)
      expect(contexts).toEqual([])
    } finally {
      stat.mockRestore()
    }
  },
)

it('adds no context when there are no observed missing inputs', () => {
  // Given
  const contexts: string[] = []
  const missing: string[] = []
  // When
  registerLoaderMissingDependencies(
    {
      _compiler: undefined,
      addMissingDependency: (path) => missing.push(path),
      addContextDependency: (path) => contexts.push(path),
    },
    [],
  )
  // Then
  expect(contexts).toEqual([])
  expect(missing).toEqual([])
})
