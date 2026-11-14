import assert from 'node:assert/strict'
import { writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { isAbsolute } from 'node:path'

const [entry, output] = process.argv.slice(2)
assert.equal(process.argv.length, 4)
assert.ok(isAbsolute(entry) && isAbsolute(output))
const require = createRequire(import.meta.url)
const api = require(entry)
const createBoundary = require('../../bindings/argument-validation.cjs')
let entered = 0
const fake = createBoundary().wrap({
  ...api,
  seedFileMap() {
    entered++
  },
})
const snapshot = () => ({
  raw: [
    'exportSheet',
    'exportClassMap',
    'exportFileMap',
    'exportCanonicalMap',
  ].map((name) => api[name]()),
  config: [
    api.getPrefix(),
    api.isDebug(),
    api.getDefaultTheme(),
    api.getUnreservedClassSlotCount(),
  ],
  css: [null, 0, 1].map((index) => api.getCss(index, false)),
})
const input = (lengths, values = []) => {
  const events = []
  let reads = 0
  const files = new Proxy(values, {
    get(target, key) {
      events.push(String(key))
      if (key !== 'length') return Reflect.get(target, key)
      const index = reads++
      if (lengths[index] === -1) return -1
      return {
        valueOf() {
          events.push('coerce:' + index)
          return lengths[index]
        },
      }
    },
  })
  return { files, events }
}
const header = [
  'length',
  'coerce:0',
  'length',
  'coerce:1',
  'length',
  'coerce:2',
]
const records = []
for (const [id, lengths] of [
  ['negative', [-1, -1, -1]],
  ['growth', [0, 0, 1]],
]) {
  for (const [host, target] of [
    ['fake', fake],
    ['actual', api],
  ]) {
    // Given: nonempty actual state and a real-array proxy with no visited indices.
    api.resetStateForTesting()
    api.seedFileMap(['src/kept.tsx'])
    const before = snapshot()
    const { files, events } = input(lengths)
    // When: the final header would extend the validated prefix.
    assert.throws(
      () => target.seedFileMap(files),
      (error) => {
        // Then: only the canonical, genuine TypeError crosses the boundary.
        assert.ok(error instanceof TypeError)
        assert.equal(error.constructor, TypeError)
        assert.equal(Object.getPrototypeOf(error), TypeError.prototype)
        assert.equal(
          error.message,
          'seedFileMap(): argument files must be an array of strings',
        )
        return true
      },
    )
    assert.deepEqual(
      events,
      id === 'negative' ? Array(3).fill('length') : header,
    )
    assert.equal(entered, 0)
    assert.deepEqual(snapshot(), before)
    records.push({ id, host, events, rejected: true, entered, unchanged: true })
  }
}
// Given: two validated strings, with only the first retained by the final header.
api.resetStateForTesting()
const shrinking = input([2, 2, 2, 2, 1], ['src/kept.tsx', 'src/dropped.tsx'])
// When: the actual exported API receives a valid shrinking copy.
const result = api.seedFileMap(shrinking.files)
// Then: no original index is reread, and only the retained path is seeded.
assert.equal(result, undefined)
assert.deepEqual(shrinking.events, [
  'length',
  'coerce:0',
  'length',
  'coerce:1',
  '0',
  'length',
  'coerce:2',
  '1',
  'length',
  'coerce:3',
  'length',
  'coerce:4',
])
const fileMap = JSON.parse(api.exportFileMap())
assert.deepEqual(fileMap, { 'src/kept.tsx': 0 })
records.push({ id: 'shrinking', events: shrinking.events, fileMap })
writeFileSync(output, JSON.stringify(records, null, 2) + '\n', { flag: 'wx' })
