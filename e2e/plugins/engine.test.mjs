import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { basename, dirname, join, sep } from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'
import { runInNewContext } from 'node:vm'

import {
  assertErrorParity,
  assertRawParity,
  assertWebpackDist,
  errorEvidence,
  initialLayerGate,
  webpackDistFiles,
} from './engine-consumer.mjs'
import {
  HostPackError,
  hostPackRealpath,
  hostPackSelection,
  verifyHostInstall,
} from './engine-fixtures.mjs'

test('host path policy equates short, expanded and junction spellings exactly', (t) => {
  const root = mkdtempSync(join(tmpdir(), 'devup host realpath '))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const target = join(root, 'target')
  const alias = join(root, 'alias')
  mkdirSync(target)
  symlinkSync(target, alias, process.platform === 'win32' ? 'junction' : 'dir')
  const expanded = realpathSync.native(target)
  assert.equal(hostPackRealpath(target), expanded)
  assert.equal(hostPackRealpath(expanded), expanded)
  assert.equal(hostPackRealpath(alias), expanded)
})

test('host path policy retries only EISDIR with the exact same input', () => {
  const calls = []
  const result = hostPackRealpath('same-input', {
    native(filename) {
      calls.push(['native', filename])
      throw Object.assign(new Error('native EISDIR'), { code: 'EISDIR' })
    },
    regular(filename) {
      calls.push(['regular', filename])
      return 'physical-path'
    },
  })
  assert.equal(result, 'physical-path')
  assert.deepEqual(calls, [
    ['native', 'same-input'],
    ['regular', 'same-input'],
  ])
})

for (const code of ['ENOENT', 'EACCES']) {
  test(`host path policy preserves ${code} without fallback`, () => {
    const failure = Object.assign(new Error('native failure'), { code })
    assert.throws(
      () =>
        hostPackRealpath('same-input', {
          native() {
            throw failure
          },
          regular() {
            assert.fail('Only EISDIR may retry')
          },
        }),
      (error) => error === failure,
    )
  })
}

for (const format of ['require', 'import']) {
  test(`built ${format} shorthand error uses its public constructor, not the other format`, async () => {
    // Given
    const cjs = createRequire(import.meta.url)(
      '../../packages/plugin-utils/dist/index.cjs',
    )
    const esm = await import('../../packages/plugin-utils/dist/index.mjs')
    const selected = format === 'require' ? cjs : esm
    const other = format === 'require' ? esm : cjs
    const invalid = () =>
      selected.normalizeShorthands({ bad: ['backgroundColour'] })
    // When / Then
    assert.throws(invalid, selected.ShorthandConfigError)
    assert.throws(
      () => assert.throws(invalid, other.ShorthandConfigError),
      assert.AssertionError,
    )
  })
}

for (const order of ['AB', 'BA']) {
  for (const releaseAt of [0, 1, 2]) {
    test(`initial gate releases ${order} admissions in filename order with CSS signal at ${releaseAt}`, () => {
      // Given
      const module = { exports: {} }
      runInNewContext(initialLayerGate, { module }, { timeout: 1000 })
      const loader = module.exports
      const released = []
      // When
      for (let index = 0; index <= order.length; index++) {
        if (index === releaseAt) loader.release()
        if (index < order.length) {
          const name = order[index]
          loader.call(
            {
              resourcePath: `C:/fixture/src/${name}.css.ts`,
              async: () => (error, source) => {
                assert.equal(error, null)
                released.push(source)
              },
            },
            name,
          )
        }
        // Then
        assert.deepEqual(
          released,
          index >= releaseAt && index >= 1 ? ['A', 'B'] : [],
        )
      }
      assert.equal(
        loader.call({ resourcePath: 'C:/fixture/src/B.css.ts' }, 'edited'),
        'edited',
      )
    })
  }
}

test('built Webpack entry and both loader formats use only the shared selector', () => {
  // Given
  const dist = fileURLToPath(
    new URL('../../packages/webpack-plugin/dist/', import.meta.url),
  )
  // When / Then
  assertWebpackDist(dist)
})

for (const name of webpackDistFiles) {
  test(`rejects a direct backend import in ${name}`, (t) => {
    // Given
    const root = mkdtempSync(join(tmpdir(), 'devup-dist-guard-'))
    t.after(() => rmSync(root, { recursive: true, force: true }))
    for (const file of webpackDistFiles) {
      writeFileSync(join(root, file), 'import "@devup-ui/plugin-utils";')
    }
    writeFileSync(
      join(root, name),
      'import "@devup-ui/plugin-utils"; require("@devup-ui/wasm");',
    )
    // When / Then
    assert.throws(() => assertWebpackDist(root), assert.AssertionError)
  })
}

for (const field of ['code', 'css', 'map', 'dependencies', 'sheet', 'error']) {
  test(`raw parity rejects a changed ${field} without normalizing it`, () => {
    // Given
    const expected = {
      code: 'x\r\n',
      css: '/* retained */.a{}',
      map: undefined,
      dependencies: ['a', 'b'],
      sheet: '{"x":1}',
      error: { name: 'Error', message: 'file.tsx:1:2' },
    }
    const actual = {
      ...expected,
      [field]: field === 'dependencies' ? ['b', 'a'] : 'different',
    }
    // When / Then
    assert.throws(
      () => assertRawParity(expected, actual),
      assert.AssertionError,
    )
  })
}

test('error parity rejects a difference only in a non-enumerable value', () => {
  // Given
  const expected = new Error('same diagnostic')
  const actual = new Error('same diagnostic')
  Object.defineProperty(expected, 'detail', { value: 'before' })
  Object.defineProperty(actual, 'detail', { value: 'after' })
  // When / Then
  assert.throws(
    () => assertErrorParity(expected, actual),
    assert.AssertionError,
  )
})

test('only Bun permits optional original creation-field presence', () => {
  const expected = new Error('same diagnostic')
  const actual = new Error('same diagnostic')
  for (const key of ['originalLine', 'originalColumn']) {
    Object.defineProperty(expected, key, {
      value: 1,
      configurable: true,
      enumerable: false,
      writable: true,
    })
    Reflect.deleteProperty(actual, key)
  }
  for (const compare of [
    () => assertErrorParity(expected, actual),
    () => assertRawParity(errorEvidence(expected), errorEvidence(actual)),
  ]) {
    if (process.versions.bun) assert.doesNotThrow(compare)
    else assert.throws(compare, assert.AssertionError)
  }
})

for (const side of ['expected', 'actual']) {
  test(`error parity rejects an unrelated key present only on ${side}`, () => {
    const expected = new Error('same diagnostic')
    const actual = new Error('same diagnostic')
    Object.defineProperty(side === 'expected' ? expected : actual, 'other', {
      value: 1,
    })
    assert.throws(
      () => assertErrorParity(expected, actual),
      assert.AssertionError,
    )
    assert.throws(
      () => assertRawParity(errorEvidence(expected), errorEvidence(actual)),
      assert.AssertionError,
    )
  })
}

test('optional creation fields retain their descriptor contract', () => {
  const expected = new Error('same diagnostic')
  const actual = new Error('same diagnostic')
  for (const [error, writable] of [
    [expected, true],
    [actual, false],
  ]) {
    Reflect.ownKeys(error)
    Reflect.get(error, 'stack')
    Object.defineProperty(error, 'originalLine', {
      value: 1,
      configurable: true,
      enumerable: false,
      writable,
    })
  }
  assert.equal(
    Object.getOwnPropertyDescriptor(actual, 'originalLine').writable,
    false,
  )
  assert.throws(
    () => assertErrorParity(expected, actual),
    assert.AssertionError,
  )
  assert.throws(
    () => assertRawParity(errorEvidence(expected), errorEvidence(actual)),
    assert.AssertionError,
  )
})

test('error parity retains the order of every other own key', () => {
  const expected = new Error('same diagnostic')
  const actual = new Error('same diagnostic')
  for (const key of ['a', 'b'])
    Object.defineProperty(expected, key, { value: 1 })
  for (const key of ['b', 'a']) Object.defineProperty(actual, key, { value: 1 })
  assert.throws(
    () => assertErrorParity(expected, actual),
    assert.AssertionError,
  )
  assert.throws(
    () => assertRawParity(errorEvidence(expected), errorEvidence(actual)),
    assert.AssertionError,
  )
})

test('error parity rejects a difference only in a descriptor flag', () => {
  // Given
  const expected = new Error('same diagnostic')
  const actual = new Error('same diagnostic')
  Object.defineProperty(expected, 'detail', { value: 1, writable: false })
  Object.defineProperty(actual, 'detail', { value: 1, writable: true })
  // When / Then
  assert.throws(
    () => assertErrorParity(expected, actual),
    assert.AssertionError,
  )
})

for (const difference of ['value', 'writable']) {
  test(`packed error receipts reject a non-enumerable ${difference} difference`, () => {
    // Given
    const expected = new Error('same diagnostic')
    const actual = new Error('same diagnostic')
    Object.defineProperty(expected, 'detail', { value: 1, writable: false })
    Object.defineProperty(actual, 'detail', {
      value: difference === 'value' ? 2 : 1,
      writable: difference === 'writable',
    })
    const before = errorEvidence(expected)
    const after = errorEvidence(actual)
    // When / Then
    assert.throws(() => assertRawParity(before, after), assert.AssertionError)
  })
}

test('error parity retains native-only public fields rather than excluding them', () => {
  // Given
  const expected = new Error('same diagnostic')
  const actual = new Error('same diagnostic')
  Object.defineProperty(actual, 'code', { value: 'GenericFailure' })
  // When / Then
  assert.throws(
    () => assertErrorParity(expected, actual),
    assert.AssertionError,
  )
})

test('error parity compares cause diagnostics recursively', () => {
  // Given
  const expected = new Error('outer', { cause: new Error('before') })
  const actual = new Error('outer', { cause: new Error('after') })
  // When / Then
  assert.throws(
    () => assertErrorParity(expected, actual),
    assert.AssertionError,
  )
})

for (const kind of ['symbol', 'function', 'accessor']) {
  test(`error parity rejects different live ${kind} identities with equal text`, () => {
    // Given
    const expected = new Error('same diagnostic')
    const actual = new Error('same diagnostic')
    const descriptor = () => {
      switch (kind) {
        case 'symbol':
          return { value: Symbol('same') }
        case 'function':
          return { value: function same() {} }
        case 'accessor':
          return {
            get: function same() {
              return 1
            },
          }
        default:
          assert.fail(`Unknown identity control ${kind}`)
      }
    }
    Object.defineProperty(expected, 'detail', descriptor())
    Object.defineProperty(actual, 'detail', descriptor())
    // When / Then
    assert.throws(
      () => assertErrorParity(expected, actual),
      assert.AssertionError,
    )
  })
}

test('error parity accepts shared live symbol keys and accessor identities', () => {
  // Given
  const key = Symbol('detail')
  const get = () => 1
  const expected = new Error('same diagnostic')
  const actual = new Error('same diagnostic')
  Object.defineProperty(expected, key, { get })
  Object.defineProperty(actual, key, { get })
  // When / Then
  assertErrorParity(expected, actual)
})

test('saved error evidence retains complete raw stack values', () => {
  // Given
  const error = new Error('raw stack')
  const stack = error.stack
  // When
  const evidence = errorEvidence(error)
  // Then
  const property = evidence.raw.properties.find(
    ({ key }) => key.type === 'string' && key.value === 'stack',
  )
  assert.equal(property.observed.value, stack)
})

function fixture(t, archiveMarker) {
  const root = mkdtempSync(join(tmpdir(), 'devup host guard '))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const directory = join(root, 'consumer')
  const host = hostPackSelection()
  const tarballs = {}
  const locations = new Map()
  const lock = { lockfileVersion: 1, packages: {} }
  const names = [
    '@devup-ui/binding',
    host.name,
    '@devup-ui/wasm',
    '@devup-ui/plugin-utils',
    '@devup-ui/react',
    '@devup-ui/webpack-plugin',
    '@devup-ui/next-plugin',
  ]
  mkdirSync(join(directory, 'node_modules/@devup-ui'), { recursive: true })
  mkdirSync(join(root, 'packs'))
  for (const [index, name] of names.entries()) {
    const source = join(root, `source-${index}`)
    const payload = join(source, 'package')
    mkdirSync(payload, { recursive: true })
    const manifest = { name, version: '0.1.0', main: 'index.js' }
    if (name === '@devup-ui/binding') {
      manifest.optionalDependencies = Object.fromEntries(
        [host.name, ...host.omitted].map((item) => [item, '0.1.0']),
      )
    }
    writeFileSync(
      join(payload, 'package.json'),
      JSON.stringify(manifest, null, 2),
    )
    writeFileSync(
      join(payload, 'index.js'),
      `/* retained ${name} */\r\nexport const value=1\r\n`,
    )
    writeFileSync(
      join(payload, 'index.js.map'),
      JSON.stringify({ sources: [name], sourcesContent: ['original\r\n'] }),
    )
    if (archiveMarker && name === '@devup-ui/react') {
      const marker = join(payload, archiveMarker)
      mkdirSync(dirname(marker), { recursive: true })
      writeFileSync(marker, 'archive-controlled marker')
    }
    const archive = join(root, 'packs', `package ${index}.tgz`)
    execFileSync('tar', ['-czf', basename(archive), '-C', source, 'package'], {
      cwd: dirname(archive),
      timeout: 30000,
    })
    const installed = join(
      directory,
      'node_modules/.bun',
      `slot-${index}`,
      'node_modules',
      name,
    )
    cpSync(payload, installed, { recursive: true })
    symlinkSync(
      installed,
      join(directory, 'node_modules', name),
      process.platform === 'win32' ? 'junction' : 'dir',
    )
    locations.set(name, installed)
    tarballs[name] = `file:${archive.split(sep).join('/')}`
    lock.packages[name] = [
      `${name}@${archive.split(sep).join('/')}`,
      {},
      `sha512-${createHash('sha512').update(readFileSync(archive)).digest('base64')}`,
    ]
  }
  for (const name of host.omitted) lock.packages[name] = [`${name}@0.1.0`, {}]
  const saveLock = () =>
    writeFileSync(join(directory, 'bun.lock'), JSON.stringify(lock, null, 2))
  saveLock()
  return {
    root,
    directory,
    locations,
    lock,
    saveLock,
    options: {
      directory,
      tarballs,
      host,
      bun: process.env.BUN_BINARY ?? 'bun',
    },
  }
}

function assertRejected(options, expected) {
  assert.throws(
    () => verifyHostInstall(options),
    (error) => {
      assert.ok(error instanceof HostPackError)
      assert.equal(error.category, expected.category)
      assert.equal(error.packageName, expected.packageName)
      return true
    },
  )
}

test('accepts all own tarballs with physical deduplication of isolated links', (t) => {
  // Given
  const fs = fixture(t)
  // When
  const receipt = verifyHostInstall(fs.options)
  // Then
  assert.equal(receipt.mode, 'host-only')
  assert.deepEqual(
    new Set(receipt.packages.map((item) => item.name)),
    new Set(Object.keys(fs.options.tarballs)),
  )
  assert.equal(receipt.packages.length, Object.keys(fs.options.tarballs).length)
  assert.equal(receipt.tarballs.length, Object.keys(fs.options.tarballs).length)
  assert.deepEqual(receipt.omitted, fs.options.host.omitted)
})

test('rejects a nested payload byte mismatch even when the root copy matches', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = '@devup-ui/react'
  const nested = join(
    fs.directory,
    'node_modules/tool/node_modules',
    packageName,
  )
  cpSync(fs.locations.get(packageName), nested, { recursive: true })
  writeFileSync(join(nested, 'index.js.map'), 'different map bytes')
  // When / Then
  assertRejected(fs.options, { category: 'byte-mismatch', packageName })
})

test('rejects byte-identical registry provenance beside a valid file record', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = '@devup-ui/react'
  cpSync(
    fs.locations.get(packageName),
    join(fs.directory, 'node_modules/tool/node_modules', packageName),
    { recursive: true },
  )
  fs.lock.packages[`tool/${packageName}`] = [
    `${packageName}@0.1.0`,
    {},
    'https://registry.npmjs.org/byte-identical.tgz',
  ]
  fs.saveLock()
  // When / Then
  assertRejected(fs.options, { category: 'source-mismatch', packageName })
})

test('rejects a byte-identical local archive from a different canonical source', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = '@devup-ui/plugin-utils'
  const other = join(fs.root, 'other source.tgz')
  const original = fs.options.tarballs[packageName].slice('file:'.length)
  cpSync(original, other)
  assert.deepEqual(readFileSync(other), readFileSync(original))
  fs.lock.packages[packageName] = [
    `${packageName}@${other.split(sep).join('/')}`,
    {},
    fs.lock.packages[packageName][2],
  ]
  fs.saveLock()
  // When / Then
  assertRejected(fs.options, { category: 'source-mismatch', packageName })
})

test('rejects a missing host sibling rather than treating optional omission as success', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = fs.options.host.name
  rmSync(join(fs.directory, 'node_modules', packageName), {
    recursive: true,
    force: true,
  })
  rmSync(fs.locations.get(packageName), { recursive: true, force: true })
  // When / Then
  assertRejected(fs.options, { category: 'missing-host', packageName })
})

test('rejects an unexpected nested Devup package outside the pack inventory', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = '@devup-ui/unexpected'
  const extra = join(
    fs.directory,
    'node_modules/tool/node_modules',
    packageName,
  )
  mkdirSync(extra, { recursive: true })
  writeFileSync(
    join(extra, 'package.json'),
    JSON.stringify({ name: packageName }),
  )
  // When / Then
  assertRejected(fs.options, { category: 'unexpected-package', packageName })
})

test('rejects an orphan host cache entry that the binding cannot resolve', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = fs.options.host.name
  rmSync(join(fs.directory, 'node_modules', packageName), {
    recursive: true,
    force: true,
  })
  // When / Then
  assertRejected(fs.options, { category: 'missing-host', packageName })
})

test('rejects an installed nonhost sibling despite optional registry metadata being allowed', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = fs.options.host.omitted[0]
  const extra = join(fs.directory, 'node_modules', packageName)
  mkdirSync(extra, { recursive: true })
  writeFileSync(
    join(extra, 'package.json'),
    JSON.stringify({ name: packageName }),
  )
  fs.lock.packages[packageName] = [`${packageName}@0.1.0`, {}]
  fs.saveLock()
  // When / Then
  assertRejected(fs.options, { category: 'unexpected-package', packageName })
})

test('accepts exact archive SRI markers on every installed Devup package', (t) => {
  // Given
  const fs = fixture(t)
  for (const [name, directory] of fs.locations) {
    writeFileSync(
      join(directory, '.bun-tag'),
      Buffer.from(fs.lock.packages[name][2]),
    )
  }
  // When
  const receipt = verifyHostInstall(fs.options)
  // Then
  assert.deepEqual(
    new Set(receipt.packages.map((item) => item.name)),
    new Set(fs.locations.keys()),
  )
})

test('rejects marker bytes with an appended newline', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = '@devup-ui/wasm'
  writeFileSync(
    join(fs.locations.get(packageName), '.bun-tag'),
    `${fs.lock.packages[packageName][2]}\n`,
  )
  // When / Then
  assertRejected(fs.options, { category: 'marker-bytes', packageName })
})

test('rejects an unknown extra file beside an attested marker', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = '@devup-ui/plugin-utils'
  const directory = fs.locations.get(packageName)
  writeFileSync(join(directory, '.bun-tag'), fs.lock.packages[packageName][2])
  writeFileSync(join(directory, 'unknown-extra'), 'not in the archive')
  // When / Then
  assertRejected(fs.options, { category: 'byte-mismatch', packageName })
})

test('rejects a marker symlink including a Windows directory junction', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = '@devup-ui/react'
  const target = join(fs.root, 'marker-target')
  mkdirSync(target)
  symlinkSync(
    target,
    join(fs.locations.get(packageName), '.bun-tag'),
    process.platform === 'win32' ? 'junction' : 'dir',
  )
  // When / Then
  assertRejected(fs.options, { category: 'marker-kind', packageName })
})

test('rejects a marker directory', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = '@devup-ui/react'
  mkdirSync(join(fs.locations.get(packageName), '.bun-tag'))
  // When / Then
  assertRejected(fs.options, { category: 'marker-kind', packageName })
})

test('rejects a nested marker even with correct archive SRI bytes', (t) => {
  // Given
  const fs = fixture(t)
  const packageName = '@devup-ui/wasm'
  const nested = join(fs.locations.get(packageName), 'nested')
  mkdirSync(nested)
  writeFileSync(join(nested, '.bun-tag'), fs.lock.packages[packageName][2])
  // When / Then
  assertRejected(fs.options, { category: 'marker-nested', packageName })
})

for (const marker of ['.bun-tag', 'nested/.bun-tag']) {
  test(`rejects an archive containing ${marker}`, (t) => {
    // Given
    const fs = fixture(t, marker)
    const packageName = '@devup-ui/react'
    // When / Then
    assertRejected(fs.options, { category: 'archive-marker', packageName })
  })
}

for (const withMarker of [false, true]) {
  test(`rejects mismatched lock integrity with marker ${withMarker}`, (t) => {
    // Given
    const fs = fixture(t)
    const packageName = '@devup-ui/wasm'
    if (withMarker) {
      writeFileSync(
        join(fs.locations.get(packageName), '.bun-tag'),
        fs.lock.packages[packageName][2],
      )
    }
    fs.lock.packages[packageName][2] =
      `sha512-${createHash('sha512').update('different archive').digest('base64')}`
    fs.saveLock()
    // When / Then
    assertRejected(fs.options, {
      category: 'lock-integrity-mismatch',
      packageName,
    })
  })

  test(`rejects missing lock integrity with marker ${withMarker}`, (t) => {
    // Given
    const fs = fixture(t)
    const packageName = '@devup-ui/wasm'
    if (withMarker) {
      writeFileSync(
        join(fs.locations.get(packageName), '.bun-tag'),
        fs.lock.packages[packageName][2],
      )
    }
    fs.lock.packages[packageName].pop()
    fs.saveLock()
    // When / Then
    assertRejected(fs.options, {
      category: 'lock-integrity-missing',
      packageName,
    })
  })
}
