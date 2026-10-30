import assert from 'node:assert/strict'
import { join, resolve } from 'node:path'
import { test } from 'node:test'

import { targets } from './matrix.mjs'
import { packageDirectory, packageName, verifyTarball } from './pack.mjs'
import { readJson, root, sha256 } from './provenance.mjs'
import { publishSelected, verifyBundle, verifySource } from './publish.mjs'

// Dispatcher UNIT fixtures, not foreign build evidence. Only npm/IO is injected.
function fixture(row) {
  const files = { 'bun.lock': 'lock-bytes', 'package.json': 'manifest-bytes' }
  const source = {
    commit: 'merge-source',
    tree: 'merge-tree',
    dirty: false,
    files,
    digest: sha256(JSON.stringify(files)),
  }
  const packs = [undefined, ...targets].map((item) => ({
    name: packageName(item),
    version: '0.1.1',
    file: item ? `${item.suffix}.tgz` : 'binding.tgz',
    integrity: `sha512-${item?.suffix || 'root'}`,
    ...(item ? { suffix: item.suffix, image: { sha256: item.suffix } } : {}),
  }))
  const bundle = {
    schema: 1,
    status: 'verified',
    mode: 'FULL nine-pack',
    run: '123/1',
    source,
    packs,
    receipts: targets.map((item) => ({
      status: 'built',
      run: '123/1',
      source,
      row: item,
      image: { sha256: item.suffix },
      original: { sha256: item.suffix },
    })),
  }
  const calls = []
  const io = {
    source: () => structuredClone(source),
    inspect: (file) =>
      structuredClone(
        packs.find((pack) => file === resolve(root, 'bundle', pack.file)),
      ),
    manifest: () => ({ name: packageName(row), version: '0.1.1' }),
    registry: (pack) => {
      calls.push(['registry', pack.name])
      return {
        name: pack.name,
        version: pack.version,
        dist: { integrity: pack.integrity },
      }
    },
    publish: (file) => calls.push(['publish', file]),
  }
  const options = {
    mode: 'publish',
    bundle,
    directory: resolve(root, 'bundle'),
    run: '123/1',
    cwd: join(root, packageDirectory(row)),
    env: {
      GITHUB_ACTIONS: 'true',
      GITHUB_EVENT_NAME: 'push',
      GITHUB_REF: 'refs/heads/main',
      GITHUB_REPOSITORY: 'dev-five-git/devup-ui',
      GITHUB_RUN_ATTEMPT: '2',
      GITHUB_WORKFLOW_REF:
        'dev-five-git/devup-ui/.github/workflows/publish.yml@refs/heads/main',
      ACTIONS_ID_TOKEN_REQUEST_URL: 'https://oidc.invalid',
      ACTIONS_ID_TOKEN_REQUEST_TOKEN: 'injected',
    },
  }
  return { options, io, calls }
}

test('publishes only the selected absolute tarball when a sibling is selected', () => {
  // Given
  const { options, io, calls } = fixture(targets[3])
  // When
  const result = publishSelected(options, io)
  // Then
  assert.deepEqual(calls, [
    ['publish', resolve(options.directory, `${targets[3].suffix}.tgz`)],
  ])
  assert.equal(result.integrity, `sha512-${targets[3].suffix}`)
})
test('requires eight registry integrities before root publication on a release-only retry', () => {
  // Given: attempt2 still consumes original123/1 payloads, not a rebuilt bundle.
  const { options, io, calls } = fixture()
  // When
  publishSelected(options, io)
  // Then
  assert.deepEqual(calls, [
    ...targets.map((row) => ['registry', packageName(row)]),
    ['publish', resolve(options.directory, 'binding.tgz')],
  ])
})
test('performs only local verification when dry-run is selected without authority', () => {
  // Given
  const { options, io, calls } = fixture()
  // When
  const result = publishSelected({ ...options, mode: 'dry-run', env: {} }, io)
  // Then
  assert.equal(result.status, 'local-validation-only')
  assert.deepEqual(calls, [])
})
for (const [key, value] of [
  ['GITHUB_ACTIONS', 'false'],
  ['GITHUB_EVENT_NAME', 'pull_request'],
  ['GITHUB_REF', 'refs/heads/other'],
  ['GITHUB_REPOSITORY', 'fork/repo'],
  [
    'GITHUB_WORKFLOW_REF',
    'dev-five-git/devup-ui/.github/workflows/other.yml@refs/heads/main',
  ],
  ['ACTIONS_ID_TOKEN_REQUEST_URL', ''],
  ['ACTIONS_ID_TOKEN_REQUEST_TOKEN', ''],
])
  test(`denies real mode when ${key} is unauthorized`, () => {
    // Given
    const { options, io, calls } = fixture()
    // When / Then
    assert.throws(() =>
      publishSelected(
        { ...options, env: { ...options.env, [key]: value } },
        io,
      ),
    )
    assert.deepEqual(calls, [])
  })
for (const [name, result] of [
  ['missing', {}],
  ['wrong version', { version: '0.1.0' }],
  ['wrong integrity', { version: '0.1.1', dist: { integrity: 'foreign' } }],
])
  test(`denies root publication when a sibling registry result is ${name}`, () => {
    // Given
    const { options, io, calls } = fixture()
    // When / Then
    assert.throws(() =>
      publishSelected(options, {
        ...io,
        registry: (pack) => ({ name: pack.name, ...result }),
      }),
    )
    assert.deepEqual(calls, [])
  })
for (const [key, value] of [
  ['commit', 'older-action-sourceSha'],
  ['tree', 'different-tree'],
  ['files', { 'bun.lock': 'changed-lock' }],
])
  test(`denies publication when current source ${key} differs`, () => {
    // Given
    const { options, io, calls } = fixture()
    // When / Then
    assert.throws(() =>
      publishSelected(options, {
        ...io,
        source: () => ({ ...options.bundle.source, [key]: value }),
      }),
    )
    assert.deepEqual(calls, [])
  })
for (const [path, value] of [
  [['options', 'bundle', 'source', 'dirty'], true],
  [['options', 'bundle', 'packs', 'length'], 8],
  [['options', 'bundle', 'receipts', 0, 'run'], '123/2'],
  [['options', 'run'], '456/1'],
  [['io', 'inspect'], () => ({ integrity: 'corrupt' })],
  [['io', 'manifest'], () => ({ name: packageName(), version: '0.1.0' })],
  [['options', 'cwd'], join(root, 'packages/react')],
])
  test(`fails before external operations when ${path.join('.')} changes`, () => {
    // Given: mutate one independent boundary value in a fresh fixture.
    const f = fixture()
    const parent = path.slice(0, -1).reduce((object, key) => object[key], f)
    parent[path.at(-1)] = value
    // When / Then
    assert.throws(() => publishSelected(f.options, f.io))
    assert.deepEqual(f.calls, [])
  })
test('tolerates the untracked action executable when all recorded source bytes match', () => {
  // Given
  const { options, io, calls } = fixture(targets[0])
  const source = options.bundle.source
  // When
  publishSelected(options, {
    ...io,
    source: () => ({
      ...source,
      dirty: true,
      files: { ...source.files, 'changepacks.exe': 'tool' },
    }),
  })
  // Then
  assert.equal(calls[0][0], 'publish')
})
test('rejects unrelated untracked inputs when the action executable is present', () => {
  // Given
  const { options } = fixture()
  const source = options.bundle.source
  // When / Then
  assert.throws(() =>
    verifySource(source, {
      ...source,
      dirty: true,
      files: { ...source.files, changepacks: 'tool', 'evil.js': 'mutation' },
    }),
  )
})
test('propagates npm errors when the injected publication fails', () => {
  // Given
  const { options, io } = fixture(targets[0])
  const failure = new Error('injected registry outage')
  // When / Then
  assert.throws(
    () =>
      publishSelected(options, {
        ...io,
        publish: () => {
          throw failure
        },
      }),
    failure,
  )
})
test('reinspects a real local tarball when the publisher payload boundary is used', () => {
  // Given: genuine host proof, explicitly not a full-nine release positive.
  const directory = process.env.NATIVE_PROOF_BUNDLE
  const bundle = readJson(join(directory, 'bundle.json'))
  const pack = bundle.packs.find((item) => item.suffix)
  const row = targets.find((item) => item.suffix === pack.suffix)
  // When
  const actual = verifyTarball(join(directory, pack.file), {
    row,
    version: pack.version,
    image: pack.image,
  })
  // Then
  assert.deepEqual(actual, pack)
})
test('rejects the genuine proof when release run identity differs', () => {
  // Given
  const directory = process.env.NATIVE_PROOF_BUNDLE
  const bundle = readJson(join(directory, 'bundle.json'))
  // When / Then
  assert.throws(() =>
    verifyBundle(bundle, {
      directory,
      source: bundle.source,
      run: 'foreign/1',
    }),
  )
})
