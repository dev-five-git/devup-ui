import assert from 'node:assert/strict'
import { basename, join, resolve } from 'node:path'

import { targets } from './matrix.mjs'
import { packageDirectory, packageName, verifyTarball } from './pack.mjs'
import {
  command,
  direct,
  readJson,
  root,
  sha256,
  sourceIdentity,
} from './provenance.mjs'

// Recheck inside the dispatcher: changepacks can checkout sourceSha after setup.
export function verifySource(expected, actual) {
  assert.equal(expected.dirty, false, 'tested source must be clean')
  assert.equal(
    actual.commit,
    expected.commit,
    'selected source commit mismatch',
  )
  assert.equal(actual.tree, expected.tree, 'selected source tree mismatch')
  assert.equal(sha256(JSON.stringify(expected.files)), expected.digest)
  for (const [path, hash] of Object.entries(expected.files))
    assert.equal(actual.files[path], hash, `source input mismatch: ${path}`)
  const extra = Object.keys(actual.files).filter(
    (path) => !Object.hasOwn(expected.files, path),
  )
  assert.ok(
    extra.every((path) => /^changepacks(?:\.exe)?$/.test(path)),
    'unexpected source inputs (only untracked changepacks CLI is tolerated)',
  )
  assert.ok(!actual.dirty || extra.length > 0, 'dirty release source')
}

export function verifyBundle(bundle, context, inspect = verifyTarball) {
  assert.equal(bundle.schema, 1)
  assert.equal(bundle.status, 'verified')
  assert.equal(bundle.mode, 'FULL nine-pack')
  assert.equal(
    bundle.run,
    context.run,
    'original producer run/attempt required',
  )
  verifySource(bundle.source, context.source)
  assert.equal(bundle.packs.length, 9, 'all nine packs required')
  assert.equal(bundle.receipts.length, 8, 'all eight producers required')
  const version = bundle.packs[0].version
  for (const row of [undefined, ...targets]) {
    const matches = bundle.packs.filter(
      (pack) => pack.name === packageName(row),
    )
    assert.equal(matches.length, 1, 'exact package set required')
    const pack = matches[0]
    assert.equal(pack.version, version, 'same release version required')
    assert.equal(pack.file, row ? `${row.suffix}.tgz` : 'binding.tgz')
    assert.equal(basename(pack.file), pack.file)
    assert.deepEqual(
      inspect(join(context.directory, pack.file), {
        row,
        version,
        image: pack.image,
      }),
      pack,
      'payload integrity/metadata mismatch',
    )
    if (row) {
      const receipts = bundle.receipts.filter(
        (receipt) => receipt.row.suffix === row.suffix,
      )
      assert.equal(receipts.length, 1, 'exact producer set required')
      const receipt = receipts[0]
      assert.equal(receipt.status, 'built')
      assert.equal(receipt.run, bundle.run, 'mixed producer attempts')
      assert.deepEqual(
        receipt.source,
        bundle.source,
        'producer source mismatch',
      )
      assert.deepEqual(receipt.row, row)
      assert.deepEqual(receipt.image, pack.image)
      assert.equal(receipt.original.sha256, pack.image.sha256)
    }
  }
  return bundle.packs
}

export function authorize(env) {
  assert.equal(env.GITHUB_ACTIONS, 'true', 'GitHub main/OIDC only')
  assert.equal(env.GITHUB_EVENT_NAME, 'push', 'main push only')
  assert.equal(env.GITHUB_REF, 'refs/heads/main', 'main push only')
  assert.equal(env.GITHUB_REPOSITORY, 'dev-five-git/devup-ui')
  assert.match(
    env.GITHUB_WORKFLOW_REF || '',
    /^dev-five-git\/devup-ui\/\.github\/workflows\/publish\.yml@refs\/heads\/main$/,
  )
  assert.ok(env.ACTIONS_ID_TOKEN_REQUEST_URL, 'OIDC URL required')
  assert.ok(env.ACTIONS_ID_TOKEN_REQUEST_TOKEN, 'OIDC token required')
}

const boundary = {
  source: () => {
    for (const flags of [[], ['--cached']])
      command('git', ['diff', ...flags, '--exit-code', 'HEAD', '--'], {
        env: { ...process.env, GIT_MASTER: '1' },
      })
    return sourceIdentity()
  },
  inspect: verifyTarball,
  manifest: (directory) => readJson(join(directory, 'package.json')),
  registry: (pack) =>
    JSON.parse(
      command('npm', [
        'view',
        `${pack.name}@${pack.version}`,
        '--json',
        '--registry=https://registry.npmjs.org',
      ]),
    ),
  publish: (file) =>
    command('npm', [
      'publish',
      file,
      '--ignore-scripts',
      '--access=public',
      '--registry=https://registry.npmjs.org',
    ]),
}

// The injected IO boundary never changes production authority or validation rules.
export function publishSelected(options, io = boundary) {
  assert.ok(['publish', 'dry-run'].includes(options.mode), 'unknown mode')
  if (options.mode === 'publish') authorize(options.env)
  const packs = verifyBundle(
    options.bundle,
    {
      directory: resolve(options.directory),
      source: io.source(),
      run: options.run,
    },
    io.inspect,
  )
  const selectedRow = [undefined, ...targets].find(
    (row) => resolve(root, packageDirectory(row)) === resolve(options.cwd),
  )
  assert.equal(
    resolve(options.cwd),
    resolve(root, packageDirectory(selectedRow)),
    'dispatcher must run from a native package directory',
  )
  const selected = packs.find((pack) => pack.name === packageName(selectedRow))
  const manifest = io.manifest(options.cwd)
  assert.equal(manifest.name, selected.name, 'selected manifest name mismatch')
  assert.equal(
    manifest.version,
    selected.version,
    'selected manifest version mismatch',
  )
  const file = resolve(options.directory, selected.file)
  if (options.mode === 'dry-run')
    return {
      status: 'local-validation-only',
      file,
      integrity: selected.integrity,
    }
  // Root can only follow all eight exact, already-published optional siblings.
  if (!selectedRow)
    for (const pack of packs.filter((item) => item.suffix)) {
      const published = io.registry(pack)
      assert.equal(
        published.name,
        pack.name,
        'registry package missing/mismatched',
      )
      assert.equal(published.version, pack.version, 'registry version mismatch')
      assert.equal(
        published.dist?.integrity,
        pack.integrity,
        'registry integrity mismatch',
      )
    }
  // No catch: arbitrary npm failures are never treated as already-published success.
  verifySource(options.bundle.source, io.source())
  assert.deepEqual(
    io.inspect(file, {
      row: selectedRow,
      version: selected.version,
      image: selected.image,
    }),
    selected,
    'selected payload changed before publication',
  )
  io.publish(file)
  return { status: 'published', file, integrity: selected.integrity }
}

if (direct(import.meta.url)) {
  const [mode, ...extra] = process.argv.slice(2)
  assert.equal(extra.length, 0, 'no forwarded publication options')
  const directory = process.env.NATIVE_RELEASE_BUNDLE
  assert.ok(directory, 'NATIVE_RELEASE_BUNDLE required outside checkout')
  console.info(
    JSON.stringify(
      publishSelected({
        mode,
        directory,
        bundle: readJson(join(directory, 'bundle.json')),
        cwd: process.cwd(),
        env: process.env,
        run: `${process.env.GITHUB_RUN_ID}/1`,
      }),
    ),
  )
}
