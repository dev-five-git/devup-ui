import assert from 'node:assert/strict'
import { test } from 'node:test'

import { controls, guarded, independent } from './owner-export-inventory.js'
import { rewriteOwnerExports } from './owner-export-wrapper.js'
import { createOwnerRuntime } from './owner-runtime.js'

const fixture = [...controls, ...guarded, ...independent]
  .map((name) => `exports.${name} = ${name};`)
  .join('\n')

for (const defect of ['missing', 'duplicate', 'unknown']) {
  test(`generation refuses ${defect} export inventory`, () => {
    // Given a generated surface with one inventory defect.
    const source =
      defect === 'missing'
        ? fixture.replace('exports.seedFileMap = seedFileMap;', '')
        : defect === 'duplicate'
          ? fixture + '\nexports.seedFileMap = seedFileMap;'
          : fixture + '\nexports.unclassified = unclassified;'
    // When postprocessing attempts to close that surface.
    // Then no partially guarded artifact is returned.
    assert.throws(() => rewriteOwnerExports(source, createOwnerRuntime))
  })
}

for (const assignment of [
  "exports['bypass'] = seedFileMap;",
  'module.exports = {};',
  'exports.bypass = () => {};',
]) {
  test('generation refuses an unrecognized public assignment shape', () => {
    // Given a public bypass outside the generated assignment grammar.
    const source = fixture + '\n' + assignment
    // When closing the public export surface.
    // Then generation fails instead of retaining the raw entry.
    assert.throws(() => rewriteOwnerExports(source, createOwnerRuntime))
  })
}

test('generation removes private assignments while retaining discoverable public assignments', () => {
  // Given the full inventory and the real wrapper template.
  // When generation installs guards.
  const result = rewriteOwnerExports(fixture, createOwnerRuntime)
  // Then controls are lexical and every guarded name has a static assignment.
  assert.equal([...result.matchAll(/^exports\._owner/gm)].length, 0)
  assert.equal(
    [...result.matchAll(/^exports\.\w+ = __ownerRuntime.wrap/gm)].length,
    guarded.length,
  )
})

test('wrapper enters before forwarding getters and leaves after exceptions', () => {
  // Given a narrow scalar control seam and a hostile generated argument read.
  const events = []
  const control = {
    _ownerFault() {
      return undefined
    },
    _ownerEnterExport(opcode) {
      events.push(['enter', opcode])
      return 1n
    },
    _ownerLeaveExport(ticket) {
      events.push(['leave', ticket])
    },
    _ownerDrain() {
      events.push(['drain'])
    },
  }
  const original = (value) => {
    events.push(['forward'])
    return value.length
  }
  const wrapped = createOwnerRuntime(control).wrap(original, 3, 'seedFileMap')
  const failure = new Error('getter')
  const value = {
    get length() {
      events.push(['getter'])
      throw failure
    },
  }
  // When the generated function fails during its first conversion.
  assert.throws(
    () => wrapped(value),
    (error) => error === failure,
  )
  // Then admission surrounds the complete conversion/error unwind.
  assert.deepEqual(events, [
    ['enter', 3],
    ['forward'],
    ['getter'],
    ['leave', 1n],
    ['drain'],
  ])
})
