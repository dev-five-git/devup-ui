import assert from 'node:assert/strict'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import { loadArtifacts, verifyArtifacts } from './collect.mjs'
import { inspectImage, targets } from './matrix.mjs'
import { packageName, validateManifest, verifyTarball } from './pack.mjs'
import { hashFile, readJson, root } from './provenance.mjs'
import { child, registry } from './registry.mjs'

// Negative controls use the real host receipt/image. No fabricated foreign positives.
const artifacts = loadArtifacts(process.env.NATIVE_PROOF_ARTIFACTS)
assert.equal(artifacts.length, 1)
const original = artifacts[0]
const row = original.receipt.row
const options = {
  run: original.receipt.run,
  source: original.receipt.source,
  allowDirty: true,
  localHostOnly: row.suffix,
}
const altered = (change) => {
  const receipt = structuredClone(original.receipt)
  change(receipt)
  return { receipt, path: original.path }
}
test('requires actual loader coverage when the dedicated Node gate runs', async () => {
  // Given
  const loader = join(root, 'bindings/devup-ui-binding/loader.test.cjs')
  // When
  const output = await child(process.execPath, [
    '--test',
    '--experimental-test-coverage',
    '--test-coverage-include=**/loader.cjs',
    '--test-coverage-lines=100',
    '--test-coverage-branches=100',
    '--test-coverage-functions=100',
    loader,
  ])
  // Then: thresholds alone accept zero files, so require the actual named row.
  assert.match(
    output.stdout,
    /loader\.cjs\s*\|\s*100\.00\s*\|\s*100\.00\s*\|\s*100\.00/,
  )
  console.info(output.stdout)
})
for (const [name, change, message] of [
  [
    'wrong run',
    (receipt) => {
      receipt.run += '-foreign'
    },
    /wrong run/,
  ],
  [
    'wrong source',
    (receipt) => {
      receipt.source.commit = 'foreign'
    },
    /wrong source/,
  ],
  [
    'wrong source inputs',
    (receipt) => {
      receipt.source.digest = 'foreign'
    },
    /wrong source/,
  ],
  [
    'unfinished build',
    (receipt) => {
      receipt.status = 'building'
    },
    /successful fresh/,
  ],
  [
    'wrong hash',
    (receipt) => {
      receipt.image.sha256 = 'wrong'
      receipt.original.sha256 = 'wrong'
    },
    /attached image hash/,
  ],
  [
    'original hash mismatch',
    (receipt) => {
      receipt.original.sha256 = 'wrong'
    },
    /original image hash/,
  ],
  [
    'wrong machine receipt',
    (receipt) => {
      receipt.image.evidence.cpu = 'other'
    },
    /actual image evidence/,
  ],
  [
    'wrong libc selector',
    (receipt) => {
      receipt.row.libc = 'musl'
    },
    /wrong target selectors/,
  ],
]) {
  test(`rejects ${name} when collecting a real artifact`, () => {
    // Given
    const artifact = altered(change)
    // When / Then
    assert.throws(() => verifyArtifacts([artifact], options), message)
  })
}
test('rejects incomplete artifacts when default full collection is requested', () => {
  // Given
  const full = { ...options, localHostOnly: undefined }
  // When / Then
  assert.throws(() => verifyArtifacts(artifacts, full), /exact artifact count/)
})
test('rejects a missing artifact when local collection is requested', () => {
  // Given / When / Then
  assert.throws(() => verifyArtifacts([], options), /exact artifact count/)
})
test('rejects duplicates when all eight slots contain one real target', () => {
  // Given
  const duplicates = Array(8).fill(original)
  // When / Then
  assert.throws(
    () => verifyArtifacts(duplicates, { ...options, localHostOnly: undefined }),
    /duplicate target/,
  )
})
test('rejects dirty source when CI policy is active', () => {
  // Given
  const dirty = altered((receipt) => {
    receipt.source.dirty = true
  })
  // When / Then
  assert.throws(
    () =>
      verifyArtifacts([dirty], {
        ...options,
        source: dirty.receipt.source,
        allowDirty: false,
      }),
    /dirty source forbidden/,
  )
})
test('rejects changed image bytes when the receipt remains unchanged', () => {
  // Given
  const directory = mkdtempSync(join(tmpdir(), 'native-corrupt-control-'))
  try {
    const path = join(directory, original.receipt.image.file)
    const bytes = readFileSync(original.path)
    bytes[bytes.length - 1] ^= 1
    writeFileSync(path, bytes)
    // When / Then
    assert.throws(
      () => verifyArtifacts([{ ...original, path }], options),
      /attached image hash/,
    )
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
})
test('rejects missing image bytes when a receipt is present', () => {
  // Given / When / Then
  assert.throws(
    () =>
      verifyArtifacts(
        [{ ...original, path: `${original.path}.absent` }],
        options,
      ),
    { code: 'ENOENT' },
  )
})
test('rejects actual wrong CPU when inspecting the host image as another architecture', () => {
  // Given
  const other = targets.find(
    (item) =>
      item.os === row.os && item.libc === row.libc && item.cpu !== row.cpu,
  )
  // When / Then
  assert.throws(() => inspectImage(original.path, other), /wrong image machine/)
})
const manifest = readJson(join(root, 'bindings/devup-ui-binding/package.json'))
manifest.optionalDependencies = Object.fromEntries(
  targets.map((item) => [packageName(item), manifest.version]),
)
for (const [name, change, message] of [
  [
    'workspace edges',
    (value) => {
      value.optionalDependencies[packageName(row)] = 'workspace:*'
    },
    /unresolved workspace/,
  ],
  [
    'mismatched optional version',
    (value) => {
      value.optionalDependencies[packageName(row)] = '0.1.999'
    },
    /Expected values/,
  ],
  [
    'install hook',
    (value) => {
      value.scripts.install = 'node evil.cjs'
    },
    /lifecycle hook/,
  ],
]) {
  test(`rejects ${name} when validating the packed manifest`, () => {
    // Given
    const value = structuredClone(manifest)
    change(value)
    // When / Then
    assert.throws(
      () => validateManifest(value, undefined, manifest.version),
      message,
    )
  })
}
test('rejects unexpected tar members when checking a non-package archive', async () => {
  // Given
  const directory = mkdtempSync(join(tmpdir(), 'native-tar-control-'))
  try {
    mkdirSync(join(directory, 'package'))
    writeFileSync(join(directory, 'package/evil.cjs'), '')
    const file = join(directory, 'bad.tgz')
    await child('tar', ['-czf', file, '-C', directory, 'package'])
    // When / Then
    assert.throws(
      () => verifyTarball(file, { version: manifest.version }),
      /exact tar members/,
    )
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
})
test('serves unchanged bytes when a real package-manager-style client requests a tarball', async () => {
  // Given
  const directory = process.env.NATIVE_PROOF_BUNDLE
  const bundle = readJson(join(directory, 'bundle.json'))
  const server = await registry(bundle, directory)
  try {
    const pack = bundle.packs[1]
    const output = join(
      mkdtempSync(join(tmpdir(), 'native-registry-control-')),
      'download.tgz',
    )
    try {
      const script = `const http=require('node:http'),fs=require('node:fs');http.get(process.argv[1],r=>{if(r.statusCode!==200)throw Error(r.statusCode);r.pipe(fs.createWriteStream(process.argv[2]))})`
      // When
      await child(process.execPath, [
        '-e',
        script,
        `${server.origin}/tarballs/${pack.file}`,
        output,
      ])
      // Then
      assert.equal(hashFile(output), pack.sha256)
    } finally {
      rmSync(join(output, '..'), { recursive: true, force: true })
    }
  } finally {
    await server.close()
  }
})
