import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, dirname, join, resolve } from 'node:path'

import { inspectImage, targets } from './matrix.mjs'
import {
  command,
  direct,
  hashFile,
  readJson,
  root,
  writeJson,
} from './provenance.mjs'

export const packageName = (row) =>
  `@devup-ui/binding${row ? `-${row.suffix}` : ''}`
export const packageDirectory = (row) =>
  `bindings/devup-ui-binding${row ? `-${row.suffix}` : ''}`
export const rootFiles = [
  'pkg/index.cjs',
  'loader.cjs',
  'index.mjs',
  'index.d.cts',
  'index.d.mts',
]
const hooks =
  /^(preinstall|install|postinstall|prepare|preprepare|postprepare|prepack|postpack|prepublish|prepublishOnly|publish|postpublish)$/
export function validateManifest(manifest, row, version) {
  assert.equal(manifest.name, packageName(row))
  assert.equal(manifest.version, version)
  assert.equal(manifest.license, 'Apache-2.0')
  assert.equal(manifest.publishConfig.access, 'public')
  assert.ok(
    !Object.keys(manifest.scripts || {}).some((key) => hooks.test(key)),
    'lifecycle hook',
  )
  assert.ok(
    !JSON.stringify(manifest).includes('workspace:'),
    'unresolved workspace version',
  )
  if (row) {
    assert.deepEqual(manifest.os, [row.os])
    assert.deepEqual(manifest.cpu, [row.cpu])
    assert.deepEqual(manifest.libc, row.libc ? [row.libc] : undefined)
    assert.equal(manifest.main, `binding.${row.suffix}.node`)
    assert.deepEqual(manifest.files, [manifest.main])
    assert.equal(manifest.dependencies, undefined)
    assert.equal(manifest.optionalDependencies, undefined)
  } else {
    assert.deepEqual(manifest.files, rootFiles)
    assert.deepEqual(Object.keys(manifest.exports), ['.'])
    assert.deepEqual(manifest.exports['.'], {
      import: { types: './index.d.mts', default: './index.mjs' },
      require: { types: './index.d.cts', default: './pkg/index.cjs' },
    })
    assert.deepEqual(
      manifest.optionalDependencies,
      Object.fromEntries(targets.map((item) => [packageName(item), version])),
    )
    assert.equal(manifest.dependencies, undefined)
  }
}
export function verifyTarball(file, expected) {
  const cwd = dirname(resolve(file))
  const members = command('tar', ['-tf', basename(file)], { cwd })
    .split(/\r?\n/)
    .filter(Boolean)
  const wanted = [
    'package/package.json',
    ...(expected.row
      ? [`package/binding.${expected.row.suffix}.node`]
      : [...rootFiles.map((name) => `package/${name}`), 'package/README.md']),
  ].sort()
  assert.deepEqual(members.slice().sort(), wanted, 'exact tar members')
  const listing = command('tar', ['-tvf', basename(file)], { cwd }).split(
    /\r?\n/,
  )
  assert.ok(
    listing.every((line) => line.startsWith('-')),
    'only regular tar members',
  )
  const content = (member) =>
    command('tar', ['-xOf', basename(file), `package/${member}`], {
      cwd,
      encoding: 'buffer',
    })
  const manifest = JSON.parse(content('package.json').toString())
  validateManifest(manifest, expected.row, expected.version)
  if (expected.row) {
    const scratch = mkdtempSync(join(tmpdir(), 'native-image-check-'))
    try {
      const image = join(scratch, manifest.main)
      writeFileSync(image, content(manifest.main))
      assert.equal(hashFile(image), expected.image.sha256, 'packed image hash')
      assert.deepEqual(
        inspectImage(image, expected.row),
        expected.image.evidence,
        'packed machine/libc',
      )
    } finally {
      rmSync(scratch, { recursive: true, force: true })
    }
  } else {
    for (const name of rootFiles) {
      const text = content(name).toString()
      assert.ok(
        !text.includes(root) && !text.includes(root.replaceAll('\\', '/')),
        'source checkout path',
      )
      assert.ok(
        !/(?:[A-Za-z]:[\\/]|file:\/\/\/|\/Users\/|\/home\/)/.test(text),
        'absolute source path',
      )
      assert.ok(!/process\.dlopen/.test(text), 'private absolute image loader')
      if (expected.workspace)
        assert.deepEqual(
          content(name),
          readFileSync(join(expected.workspace, packageDirectory(), name)),
          'packed source bytes',
        )
    }
  }
  const bytes = readFileSync(file)
  return {
    file: basename(file),
    name: manifest.name,
    version: manifest.version,
    sha256: hashFile(file),
    integrity: `sha512-${createHash('sha512').update(bytes).digest('base64')}`,
    bytes: bytes.length,
    members,
    manifest,
    ...(expected.row
      ? { suffix: expected.row.suffix, image: expected.image }
      : {}),
  }
}

// An isolated workspace supplies all eight manifests for Bun's workspace:* rewrite;
// only actual artifact rows receive images and are packed. Never forge foreign images.
export function prepareWorkspace(options) {
  const workspace = mkdtempSync(join(tmpdir(), 'native-pack-workspace-'))
  writeJson(join(workspace, 'package.json'), {
    name: 'native-proof-fixture',
    private: true,
    workspaces: ['bindings/*'],
  })
  for (const row of [undefined, ...targets]) {
    const directory = join(workspace, packageDirectory(row))
    mkdirSync(directory, { recursive: true })
    const manifest = readJson(join(root, packageDirectory(row), 'package.json'))
    if (options.version) manifest.version = options.version
    writeJson(join(directory, 'package.json'), manifest)
    if (!row) {
      for (const name of [...rootFiles, 'README.md']) {
        mkdirSync(dirname(join(directory, name)), { recursive: true })
        copyFileSync(
          join(root, packageDirectory(), name),
          join(directory, name),
        )
      }
    }
  }
  for (const artifact of options.artifacts || []) {
    copyFileSync(
      artifact.path,
      join(
        workspace,
        packageDirectory(artifact.receipt.row),
        artifact.receipt.image.file,
      ),
    )
  }
  return workspace
}
export function packOne(workspace, options) {
  const file = resolve(
    options.out,
    options.row ? `${options.row.suffix}.tgz` : 'binding.tgz',
  )
  command('bun', ['pm', 'pack', '--ignore-scripts', '--filename', file], {
    cwd: join(workspace, packageDirectory(options.row)),
  })
  return verifyTarball(file, { ...options, workspace })
}
export function versionProof(out) {
  mkdirSync(out, { recursive: false })
  const results = []
  for (const version of ['0.1.0', '0.1.1']) {
    const workspace = prepareWorkspace({ version })
    try {
      const destination = join(out, version)
      mkdirSync(destination)
      const packed = packOne(workspace, { out: destination, version })
      results.push({
        version,
        optionalDependencies: packed.manifest.optionalDependencies,
        sha256: packed.sha256,
      })
    } finally {
      rmSync(workspace, { recursive: true, force: true })
    }
  }
  writeJson(join(out, 'version-proof.json'), {
    kind: 'VERSION/PACK only; no foreign execution',
    results,
  })
  return results
}
if (direct(import.meta.url)) {
  assert.equal(process.argv[2], '--version-proof')
  console.info(JSON.stringify(versionProof(resolve(process.argv[3]))))
}
