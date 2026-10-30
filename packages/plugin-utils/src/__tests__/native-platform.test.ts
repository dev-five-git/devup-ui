import { randomUUID } from 'node:crypto'
import { readFileSync } from 'node:fs'
import Module, { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'bun:test'

import { compilerRealpath, loadCompiler } from '../compiler'
import { NativePlatformError, nativePlatformPackage } from '../native-platform'

const matrixUrl = new URL(
  '../../../../e2e/binding/package/matrix.mjs',
  import.meta.url,
)
const matrixProcess = Bun.spawn(
  [
    'node',
    '--input-type=module',
    '--eval',
    `import { targets } from ${JSON.stringify(matrixUrl.href)}; console.log(JSON.stringify(targets))`,
  ],
  { stdout: 'pipe', stderr: 'pipe', timeout: 10000 },
)
const [matrixStatus, matrixText, matrixError] = await Promise.all([
  matrixProcess.exited,
  new Response(matrixProcess.stdout).text(),
  new Response(matrixProcess.stderr).text(),
])
if (matrixStatus !== 0) throw new Error(matrixError)
const parsed: unknown = JSON.parse(matrixText)
if (!Array.isArray(parsed))
  throw new TypeError('Canonical target table must be an array')
const rows: readonly unknown[] = parsed
const targets = rows.map((row) => {
  if (
    typeof row !== 'object' ||
    row === null ||
    !('suffix' in row) ||
    typeof row.suffix !== 'string' ||
    !('os' in row) ||
    typeof row.os !== 'string' ||
    !('cpu' in row) ||
    typeof row.cpu !== 'string'
  )
    throw new TypeError('Invalid canonical target row')
  const libc = 'libc' in row ? row.libc : undefined
  if (libc !== undefined && libc !== 'glibc' && libc !== 'musl') {
    throw new TypeError('Invalid canonical target libc')
  }
  return { suffix: row.suffix, os: row.os, cpu: row.cpu, libc }
})

const ownRequire = createRequire(
  compilerRealpath(fileURLToPath(import.meta.url)),
)
const authoredLoader = compilerRealpath(
  fileURLToPath(
    new URL(
      '../../../../bindings/devup-ui-binding/loader.cjs',
      import.meta.url,
    ),
  ),
)
const hostPackage = nativePlatformPackage({
  platform: process.platform,
  arch: process.arch,
  report: process.report.getReport,
})
const hostAddon: unknown = ownRequire(authoredLoader)
const hostCompiler = loadCompiler('native')
const hostFunctions = Object.entries(hostCompiler)
type LoaderTarget = Pick<(typeof targets)[number], 'os' | 'cpu' | 'libc'>
type ResolverControl = 'ordinary' | 'missing' | 'other'

function targetReport(row: LoaderTarget) {
  return { header: row.libc === 'glibc' ? { glibcVersionRuntime: '2.39' } : {} }
}

// Keep the complete override transaction synchronous: only the authored loader is
// evicted, never its public wrapper, platform sibling or native backend.
function requireAuthoredLoader(
  row: LoaderTarget,
  control: ResolverControl = 'ordinary',
) {
  const descriptors = (['platform', 'arch', 'report'] as const).map((key) => ({
    key,
    descriptor: Object.getOwnPropertyDescriptor(process, key),
  }))
  const cacheDescriptor = Object.getOwnPropertyDescriptor(
    ownRequire.cache,
    authoredLoader,
  )
  const previous = ownRequire.cache[authoredLoader]
  const original: unknown = Reflect.get(Module, '_resolveFilename')
  if (typeof original !== 'function')
    throw new TypeError('Expected Node filename resolver')
  const originalResolver = original
  const manifest =
    control === 'ordinary' ? undefined : `${hostPackage}/package.json`
  let reports = 0
  let resolverCalls = 0
  let result: unknown
  let failure: Error | undefined
  let resolverFailure: Error | undefined

  if (control === 'other') {
    try {
      readFileSync(compilerRealpath(import.meta.dirname))
    } catch (error) {
      if (!(error instanceof Error)) throw error
      resolverFailure = error
    }
    expect(resolverFailure).toBeInstanceOf(Error)
    expect(
      resolverFailure && 'code' in resolverFailure
        ? resolverFailure.code
        : undefined,
    ).toBe('EISDIR')
  }

  // This is Node's concrete private resolver signature, not a namespace mock.
  // Its four arguments must pass through unchanged on every non-target request.
  function resolveFilename(
    this: unknown,
    ...args: [
      request: string,
      parent: NodeJS.Module | null | undefined,
      isMain?: boolean,
      options?: { paths?: string[] },
    ]
  ): unknown {
    const [request, parent] = args
    if (
      request === manifest &&
      typeof parent?.filename === 'string' &&
      compilerRealpath(parent.filename) === authoredLoader
    ) {
      resolverCalls++
      switch (control) {
        case 'missing':
          try {
            return Reflect.apply(originalResolver, this, [
              `${hostPackage}-w74t-${randomUUID()}`,
              ...args.slice(1),
            ])
          } catch (error) {
            if (!(error instanceof Error)) throw error
            resolverFailure = error
            throw error
          }
        case 'other':
          throw resolverFailure
        case 'ordinary':
          break
        default: {
          const unreachable: never = control
          return unreachable
        }
      }
    }
    const resolved: unknown = Reflect.apply(originalResolver, this, args)
    return resolved
  }

  try {
    Object.defineProperty(process, 'platform', {
      configurable: true,
      value: row.os,
    })
    Object.defineProperty(process, 'arch', {
      configurable: true,
      value: row.cpu,
    })
    Object.defineProperty(process, 'report', {
      configurable: true,
      value: {
        getReport() {
          reports++
          return targetReport(row)
        },
      },
    })
    delete ownRequire.cache[authoredLoader]
    if (control !== 'ordinary')
      Reflect.set(Module, '_resolveFilename', resolveFilename)
    try {
      result = ownRequire(authoredLoader)
    } catch (error) {
      if (!(error instanceof Error)) throw error
      failure = error
    }
  } finally {
    if (control !== 'ordinary')
      Reflect.set(Module, '_resolveFilename', original)
    delete ownRequire.cache[authoredLoader]
    if (previous) ownRequire.cache[authoredLoader] = previous
    for (const { key, descriptor } of descriptors) {
      if (descriptor) Object.defineProperty(process, key, descriptor)
      else Reflect.deleteProperty(process, key)
    }
    expect(Reflect.get(Module, '_resolveFilename')).toBe(original)
    expect(ownRequire.cache[authoredLoader]).toBe(previous)
    expect(
      Object.getOwnPropertyDescriptor(ownRequire.cache, authoredLoader),
    ).toEqual(cacheDescriptor)
    for (const { key, descriptor } of descriptors) {
      expect(Object.getOwnPropertyDescriptor(process, key)).toEqual(descriptor)
    }
    const restoredCompiler = loadCompiler('native')
    expect(restoredCompiler).toBe(hostCompiler)
    for (const [name, value] of hostFunctions) {
      expect(Reflect.get(restoredCompiler, name)).toBe(value)
    }
  }
  expect(reports).toBe(row.os === 'linux' ? 1 : 0)
  return { result, failure, resolverFailure, resolverCalls }
}

describe('authored native loader drift controls', () => {
  for (const row of targets) {
    it(`actually selects the helper package for ${row.suffix}`, () => {
      // Given
      const selected = nativePlatformPackage({
        platform: row.os,
        arch: row.cpu,
        report: () => targetReport(row),
      })
      // When
      const { result, failure } = requireAuthoredLoader(row)
      // Then: linked foreign manifests reach final require, not the optional catch.
      expect(selected).toBe(`@devup-ui/binding-${row.suffix}`)
      if (selected === hostPackage) {
        expect(failure).toBeUndefined()
        expect(result).toBe(hostAddon)
      } else {
        expect(failure).toBeInstanceOf(Error)
        expect(failure?.message).toContain(selected)
        expect(failure?.message).not.toContain('Missing optional dependency')
      }
    })
  }

  it('actually rejects unsupported freebsd/x64 before reporting', () => {
    // Given
    const row = { os: 'freebsd', cpu: 'x64', libc: undefined }
    // When
    const { failure } = requireAuthoredLoader(row)
    // Then
    expect(failure?.message).toBe(
      'Unsupported @devup-ui/binding target: freebsd/x64/none',
    )
  })

  for (const control of ['missing', 'other'] as const) {
    it(`preserves the genuine ${control} resolver error for the host manifest`, () => {
      // Given
      const row = targets.find(
        (target) => `@devup-ui/binding-${target.suffix}` === hostPackage,
      )
      if (!row)
        throw new TypeError(
          'Host package must belong to the canonical target table',
        )
      // When
      const { failure, resolverFailure, resolverCalls } = requireAuthoredLoader(
        row,
        control,
      )
      // Then
      expect(resolverCalls).toBe(1)
      expect(resolverFailure).toBeInstanceOf(Error)
      switch (control) {
        case 'missing':
          expect(
            resolverFailure && 'code' in resolverFailure
              ? resolverFailure.code
              : undefined,
          ).toBe('MODULE_NOT_FOUND')
          expect(failure?.message).toBe(
            `Missing optional dependency ${hostPackage}. Reinstall @devup-ui/binding with optional dependencies enabled (do not use --omit=optional).`,
          )
          expect(failure?.cause).toBe(resolverFailure)
          break
        case 'other':
          expect(failure).toBe(resolverFailure)
          break
        default: {
          const unreachable: never = control
          throw new TypeError(`Unexpected resolver control: ${unreachable}`)
        }
      }
    })
  }
})

describe('native diagnostic platform selection', () => {
  it('supports exactly the packages in the canonical table', () => {
    // Given
    const supported = ['win32', 'darwin', 'linux'].flatMap((platform) =>
      ['x64', 'arm64'].flatMap((arch) =>
        (platform === 'linux' ? [false, true] : [false]).map((gnu) =>
          nativePlatformPackage({
            platform,
            arch,
            report: () => ({
              header: gnu ? { glibcVersionRuntime: '2.39' } : {},
            }),
          }),
        ),
      ),
    )
    // When / Then
    expect(new Set(supported)).toEqual(
      new Set(targets.map((row) => `@devup-ui/binding-${row.suffix}`)),
    )
  })

  for (const row of targets) {
    it(`matches the canonical package for ${row.suffix}`, () => {
      // Given
      let reports = 0
      const host = {
        platform: row.os,
        arch: row.cpu,
        report() {
          reports++
          return {
            header: row.libc === 'glibc' ? { glibcVersionRuntime: '2.39' } : {},
          }
        },
      }
      // When
      const name = nativePlatformPackage(host)
      // Then
      expect(name).toBe(`@devup-ui/binding-${row.suffix}`)
      expect(reports).toBe(row.os === 'linux' ? 1 : 0)
    })
  }

  it.each([
    ['freebsd', 'x64'],
    ['win32', 'ia32'],
    ['linux', 'riscv64'],
  ])(
    'fails explicitly before reporting for unsupported %s/%s',
    (platform, arch) => {
      // Given
      let reports = 0
      const host = {
        platform,
        arch,
        report() {
          reports++
          return { header: {} }
        },
      }
      // When / Then
      expect(() => nativePlatformPackage(host)).toThrow(NativePlatformError)
      expect(reports).toBe(0)
    },
  )

  it('retains a report failure instead of guessing musl', () => {
    // Given
    const failure = new Error('report unavailable')
    const host = {
      platform: 'linux',
      arch: 'x64',
      report() {
        throw failure
      },
    }
    // When / Then
    expect(() => nativePlatformPackage(host)).toThrow(
      expect.objectContaining({
        name: 'NativePlatformError',
        cause: failure,
      }),
    )
  })

  it('retains an inaccessible report header as the platform failure cause', () => {
    // Given
    const failure = new Error('header unavailable')
    const host = {
      platform: 'linux',
      arch: 'x64',
      report: () => ({
        get header() {
          throw failure
        },
      }),
    }
    // When / Then
    expect(() => nativePlatformPackage(host)).toThrow(
      expect.objectContaining({
        name: 'NativePlatformError',
        cause: failure,
      }),
    )
  })

  it.each([null, 'report text', {}, { header: null }, { header: 'bad' }])(
    'rejects malformed report %j without guessing a package',
    (report) => {
      // Given
      const host = { platform: 'linux', arch: 'arm64', report: () => report }
      // When / Then
      expect(() => nativePlatformPackage(host)).toThrow(NativePlatformError)
    },
  )

  it.each([undefined, '', false, 0])(
    'matches the loader truthiness rule for %j',
    (glibcVersionRuntime) => {
      // Given
      const host = {
        platform: 'linux',
        arch: 'x64',
        report: () => ({ header: { glibcVersionRuntime } }),
      }
      // When
      const name = nativePlatformPackage(host)
      // Then
      expect(name).toBe('@devup-ui/binding-linux-x64-musl')
    },
  )
})
