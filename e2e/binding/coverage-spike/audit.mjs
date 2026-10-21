import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

export function audit(entry, result, denominator) {
  const scratch = dirname(fileURLToPath(import.meta.url))
  const negative = entry.name === 'negative'
  const json = join(entry.reports, 'tarpaulin-report.json')
  const report = JSON.parse(readFileSync(json, 'utf8'))
  entry.files = report.files
    .map((file) => {
      const path = join(...file.path)
      assert.equal(
        readFileSync(path, 'utf8'),
        file.content,
        'Reported source differs',
      )
      const local = relative(scratch, path).replaceAll('\\', '/')
      assert.ok(
        Object.hasOwn(entry.sourceHashes, local),
        'Unexpected report source',
      )
      return {
        path: local,
        sha256: entry.sourceHashes[local],
        covered: file.covered,
        coverable: file.coverable,
        lines: file.traces
          .map((trace) => ({ line: trace.line, hit: trace.stats.Line > 0 }))
          .sort((a, b) => a.line - b.line),
      }
    })
    .sort((a, b) => a.path.localeCompare(b.path))
  assert.equal(result.status, negative ? 1 : 0, 'Unexpected tarpaulin exit')
  assert.match(result.stdout, /test result: ok\. 1 passed; 0 failed/)
  if (negative) {
    assert.match(
      result.stdout + result.stderr,
      /Coverage is below the failure threshold/,
    )
  }
  entry.host = JSON.parse(
    result.stdout.match(/\{"event":"W74a.host"[^\n]*\}/)?.[0] ?? 'null',
  )
  assert.equal(entry.host.loadedAddon, entry.image)
  assert.equal(entry.host.requestedAddon, entry.image)
  assert.equal(entry.host.control, negative ? 'negative' : 'positive')
  assert.match(entry.host.LLVM_PROFILE_FILE, /%m.*%p|%p.*%m/)
  assert.ok(entry.host.LLVM_PROFILE_FILE.startsWith(entry.target))
  assert.ok(entry.profiles.filter((profile) => profile.bytes > 0).length >= 2)
  const current = entry.files.map(({ path, sha256, coverable, lines }) => ({
    path,
    sha256,
    coverable,
    lines: lines.map(({ line }) => line),
  }))
  if (denominator) {
    assert.deepEqual(current, denominator, 'Source/line denominator changed')
  }
  for (const [path, expected] of [
    ['addon/lib.rs', 17],
    ['core.rs', 2],
  ]) {
    const file = entry.files.find((item) => item.path === path)
    assert.ok(file, `${path}: missing from coverage`)
    assert.equal(
      file.coverable,
      expected,
      `${path}: mapping discrepancy; inspect raw evidence, do not weaken gate`,
    )
    assert.equal(file.lines.length, expected)
    const omitted = negative && path === 'addon/lib.rs' ? [55, 56, 57] : []
    assert.equal(file.covered, expected - omitted.length)
    for (const { line, hit } of file.lines) {
      assert.equal(hit, !omitted.includes(line))
    }
    assert.deepEqual(
      file.lines.map(({ line }) => line),
      path === 'addon/lib.rs'
        ? [9, 10, 11, 12, 18, 19, 32, 33, 39, 40, 41, 42, 47, 48, 55, 56, 57]
        : [7, 8],
    )
  }
  for (const path of ['addon/build.rs', 'native_host.rs']) {
    assert.ok(
      entry.files.some((file) => file.path === path),
      `${path}: scaffolding disappeared`,
    )
  }
  return current
}
