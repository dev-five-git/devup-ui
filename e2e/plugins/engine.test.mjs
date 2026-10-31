import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, dirname, join, sep } from 'node:path'
import { test } from 'node:test'

import {
  HostPackError,
  hostPackSelection,
  verifyHostInstall,
} from './engine-fixtures.mjs'

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
