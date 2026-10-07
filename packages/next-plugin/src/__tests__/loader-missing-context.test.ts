import * as fs from 'node:fs'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { registerLoaderMissingDependencies } from '../loader-resolution-watch'
import { MdxFreshnessError } from '../mdx-source-freshness'

it('tracks exact absent probes without recursively subscribing their ancestors', async () => {
  // Given
  const root = mkdtempSync(join(tmpdir(), 'devup-next-missing-'))
  const outside = join(root, 'outside')
  mkdirSync(outside)
  writeFileSync(join(outside, 'file'), '')
  const paths = [
    join(root, 'tsconfig.json'),
    join(outside, 'earlier.js'),
    join(outside, 'absent', 'index.ts'),
    join(outside, 'file', 'deep', 'nested', 'index.cjs'),
  ]
  const missing: string[] = []
  const contexts: string[] = []
  const read = spyOn(fs, 'readFile')
  const context = {
    _compiler: undefined,
    fs,
    resourcePath: join(root, 'app/page.jsx'),
    addMissingDependency: (path: string) => missing.push(path),
    addContextDependency: (path: string) => contexts.push(path),
  }
  try {
    // When
    await registerLoaderMissingDependencies(context, paths)
    // Then
    expect(missing).toEqual(paths)
    expect(read.mock.calls.map(([path]) => path)).toEqual(paths)
    expect(contexts).toEqual([])
  } finally {
    read.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
})

it.each(['ENOENT', 'ENOTDIR', 'EACCES', 'EIO'])(
  'preserves the exact-read %s outcome instead of probing ancestors',
  async (code) => {
    // Given
    const failure = Object.assign(new Error('exact read failed'), { code })
    const read = spyOn(fs, 'readFile').mockImplementation(
      Object.assign(
        (...args: unknown[]) => {
          const callback = args.at(-1)
          if (typeof callback !== 'function')
            throw new TypeError('Missing callback')
          queueMicrotask(() => Reflect.apply(callback, undefined, [failure]))
        },
        { __promisify__: fs.readFile.__promisify__ },
      ),
    )
    const context = {
      _compiler: undefined,
      fs,
      resourcePath: 'page.jsx',
      addMissingDependency: () => undefined,
    }
    try {
      // When
      const result = registerLoaderMissingDependencies(context, [
        join('outside', 'missing', 'index.cjs'),
      ])
      // Then
      if (code === 'ENOENT' || code === 'ENOTDIR')
        await expect(result).resolves.toBeUndefined()
      else await expect(result).rejects.toBe(failure)
      expect(read).toHaveBeenCalled()
    } finally {
      read.mockRestore()
    }
  },
)

it('refuses stale delivery when an observed missing probe is now readable', async () => {
  // Given
  const root = mkdtempSync(join(tmpdir(), 'devup-next-readable-'))
  const path = join(root, 'earlier.js')
  writeFileSync(path, 'export const color = "blue"')
  const context = {
    _compiler: undefined,
    fs,
    resourcePath: join(root, 'page.jsx'),
    addMissingDependency: () => undefined,
  }
  try {
    // When
    const result = registerLoaderMissingDependencies(context, [path])
    // Then
    await expect(result).rejects.toBeInstanceOf(MdxFreshnessError)
    await expect(result).rejects.toMatchObject({
      filename: context.resourcePath,
      input: { kind: 'missing', path },
    })
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('waits for the receiving filesystem callback before settling missing registration', async () => {
  // Given
  let complete: (() => void) | undefined
  const read = spyOn(fs, 'readFile').mockImplementation(
    Object.assign(
      (...args: unknown[]) => {
        const callback = args.at(-1)
        if (typeof callback !== 'function')
          throw new TypeError('Missing callback')
        complete = () =>
          Reflect.apply(callback, undefined, [
            Object.assign(new Error('absent'), { code: 'ENOENT' }),
          ])
      },
      { __promisify__: fs.readFile.__promisify__ },
    ),
  )
  let settled = false
  const context = {
    _compiler: undefined,
    fs,
    resourcePath: 'page.jsx',
    addMissingDependency: () => undefined,
  }
  try {
    // When
    const result = Promise.resolve(
      registerLoaderMissingDependencies(context, ['absent.js']),
    ).then(() => {
      settled = true
    })
    await Promise.resolve()
    // Then
    expect(settled).toBe(false)
    if (!complete) throw new TypeError('No exact read requested')
    complete()
    await result
    expect(settled).toBe(true)
  } finally {
    read.mockRestore()
  }
})

it('refuses an unavailable tracked filesystem without a context fallback', async () => {
  // Given
  const context = {
    _compiler: undefined,
    fs,
    resourcePath: 'page.jsx',
    addMissingDependency: () => undefined,
  }
  Object.defineProperty(context, 'fs', { value: undefined })
  // When / Then
  await expect(
    registerLoaderMissingDependencies(context, ['absent.js']),
  ).rejects.toBeInstanceOf(TypeError)
})

it('does not require filesystem capabilities when no missing input was observed', async () => {
  // Given
  const missing: string[] = []
  const context = {
    _compiler: undefined,
    fs,
    resourcePath: 'page.jsx',
    addMissingDependency: (path: string) => missing.push(path),
  }
  Object.defineProperty(context, 'fs', { value: undefined })
  // When
  await registerLoaderMissingDependencies(context, [])
  // Then
  expect(missing).toEqual([])
})
