import assert from 'node:assert/strict'
import { mkdirSync, readdirSync, rmSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { inspectImage, targets } from './matrix.mjs'
import { packOne, prepareWorkspace } from './pack.mjs'
import {
  direct,
  hashFile,
  readJson,
  root,
  sourceIdentity,
  writeJson,
} from './provenance.mjs'

export function verifyArtifacts(artifacts, options) {
  const expected = options.localHostOnly
    ? targets.filter((row) => row.suffix === options.localHostOnly)
    : targets
  assert.ok(expected.length, 'unknown local target')
  assert.equal(
    artifacts.length,
    expected.length,
    'exact artifact count required (default eight)',
  )
  const seen = new Set()
  for (const { receipt, path } of artifacts) {
    assert.equal(receipt.schema, 1)
    assert.equal(
      receipt.status,
      'built',
      'successful fresh build receipt required',
    )
    assert.equal(receipt.run, options.run, 'wrong run')
    assert.deepEqual(receipt.source, options.source, 'wrong source')
    assert.ok(
      options.allowDirty || !receipt.source.dirty,
      'dirty source forbidden',
    )
    assert.equal(receipt.profile, 'native')
    const row = expected.find((item) => item.target === receipt.row.target)
    assert.ok(row, 'unexpected target')
    assert.deepEqual(receipt.row, row, 'wrong target selectors/libc')
    assert.ok(!seen.has(row.target), 'duplicate target')
    seen.add(row.target)
    assert.equal(receipt.image.file, `binding.${row.suffix}.node`)
    assert.equal(
      receipt.image.sha256,
      receipt.original.sha256,
      'original image hash mismatch',
    )
    assert.equal(
      hashFile(path),
      receipt.image.sha256,
      'attached image hash mismatch',
    )
    assert.deepEqual(
      inspectImage(path, row),
      receipt.image.evidence,
      'actual image evidence mismatch',
    )
  }
  if (options.localHostOnly) {
    assert.equal(
      expected[0].os,
      process.platform,
      'partial mode is local-host-only',
    )
    assert.equal(
      expected[0].cpu,
      process.arch,
      'partial mode is local-host-only',
    )
    if (process.platform === 'linux')
      assert.equal(
        expected[0].libc,
        process.report.getReport().header.glibcVersionRuntime
          ? 'glibc'
          : 'musl',
      )
  }
  return artifacts
}
export function loadArtifacts(directory) {
  return readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => {
      const folder = join(directory, entry.name)
      const receipt = readJson(join(folder, 'receipt.json'))
      assert.ok(
        targets.some(
          (row) => receipt.image?.file === `binding.${row.suffix}.node`,
        ),
        'unsafe image filename',
      )
      return { receipt, path: join(folder, receipt.image.file) }
    })
}
export function collect(options) {
  mkdirSync(options.out, { recursive: false })
  const report = {
    schema: 1,
    status: 'validating',
    mode: options.localHostOnly
      ? 'PARTIAL local-host-only two-pack'
      : 'FULL nine-pack',
    run: options.run,
  }
  let workspace
  try {
    report.source = sourceIdentity()
    const artifacts = verifyArtifacts(loadArtifacts(options.artifacts), {
      ...options,
      source: report.source,
    })
    report.receipts = artifacts.map(({ receipt }) => receipt)
    const version = readJson(
      join(root, 'bindings/devup-ui-binding/package.json'),
    ).version
    workspace = prepareWorkspace({ artifacts })
    report.packs = [packOne(workspace, { out: options.out, version })]
    for (const { receipt } of artifacts)
      report.packs.push(
        packOne(workspace, {
          out: options.out,
          version,
          row: receipt.row,
          image: receipt.image,
        }),
      )
    assert.deepEqual(
      sourceIdentity(),
      report.source,
      'source changed during collection',
    )
    report.status = 'verified'
    return report
  } catch (error) {
    report.status = 'failed'
    report.error = { message: error.message, stack: error.stack }
    throw error
  } finally {
    if (workspace) rmSync(workspace, { recursive: true, force: true })
    writeJson(join(options.out, 'bundle.json'), report)
  }
}
if (direct(import.meta.url)) {
  const [artifacts, out, run, ...flags] = process.argv.slice(2)
  const hostFlag = flags.find((flag) => flag.startsWith('--local-host-only='))
  assert.ok(
    flags.every((flag) => flag === '--allow-dirty' || flag === hostFlag),
    'unknown collector flag',
  )
  assert.ok(run, 'run required')
  collect({
    artifacts: resolve(artifacts),
    out: resolve(out),
    run,
    allowDirty: flags.includes('--allow-dirty'),
    localHostOnly: hostFlag?.split('=')[1],
  })
}
