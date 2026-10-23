import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { fixture, record, red } from './audit-fixture'

function run(root: string, order: string) {
  const receipt = join(root, `entry-revisions-${order}.json`)
  const child = spawnSync(
    'node',
    [
      join(import.meta.dir, 'entry-revisions.cjs'),
      root,
      'union',
      order,
      receipt,
    ],
    { encoding: 'utf8', timeout: 60000 },
  )
  assert.equal(child.status, 0, child.stdout + child.stderr)
  const result: unknown = JSON.parse(readFileSync(receipt, 'utf8'))
  record(result)
  console.info(JSON.stringify({ entryRevisionReceipt: order, result }))
  return result
}

it('replaces final compiler revisions without losing the other environment before native extraction', () => {
  // Given final tools.rspack entries outside D9 and one superseded physical source.
  using fs = fixture()
  fs.put('devup.json', '{}')
  fs.put('src/entry.js', 'console.log("early-entry");')
  fs.put('outside/A.js', red)
  fs.put('outside/B.js', red.replace('red', 'blue'))
  fs.put('revised/A.js', red.replace('red', 'green'))
  fs.put('outside/stale.js', red.replace('red', 'chartreuse'))
  // When two native compiler generations each build both environments in AB/BA order.
  const ab = run(fs.root, 'AB')
  const ba = run(fs.root, 'BA')
  // Then current batches replace old entries, preserve the union and keep raw output equal.
  expect(ab.beforeBuild).toBe(0)
  expect(ba.beforeBuild).toBe(0)
  expect(ab.entries).toEqual(ba.entries)
  assert(Array.isArray(ab.revisions) && Array.isArray(ba.revisions))
  expect(ab.revisions).toHaveLength(2)
  expect(ba.revisions).toHaveLength(2)
  for (const [index, left] of ab.revisions.entries()) {
    const right: unknown = ba.revisions[index]
    record(left)
    record(right)
    expect(left.assets).toEqual(right.assets)
    expect(left.css).toEqual(right.css)
    expect(left.classes).toEqual(right.classes)
    expect(left.files).toEqual(right.files)
    for (const result of [left, right]) {
      expect(result.count).toBe(0)
      assert(Array.isArray(result.batches) && Array.isArray(result.extracts))
      expect(result.batches).toHaveLength(1)
      expect(result.extracts.length).toBeGreaterThan(0)
      const batch: unknown = result.batches[0]
      record(batch)
      assert(Array.isArray(batch.rows))
      const names = batch.rows.map((row: unknown) => {
        assert(Array.isArray(row) && typeof row[0] === 'string')
        return row[0]
      })
      expect(names).toContain(join(fs.root, 'outside/B.js'))
      expect(names).toContain(
        join(fs.root, index === 0 ? 'outside/A.js' : 'revised/A.js'),
      )
      expect(names).not.toContain(
        join(fs.root, index === 0 ? 'revised/A.js' : 'outside/A.js'),
      )
      expect(names).not.toContain(join(fs.root, 'outside/stale.js'))
      for (const output of result.extracts) {
        record(output)
        expect(output.before).toBe(0)
        expect(output.after).toBe(0)
      }
    }
  }
}, 180000)
