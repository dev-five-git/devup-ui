import assert from 'node:assert/strict'
import { runInNewContext } from 'node:vm'

import { expect, it } from 'bun:test'

import { buildOperationSite } from '../build-operation-site'

for (const filename of [
  'C:\\src\\caller.js',
  'C:\\src\\owner (copy)\\caller.js',
  'file:///src/owner (copy)/caller.js',
]) {
  for (const source of [
    'site("configure")',
    'function invoke() { return site("configure") }; invoke()',
  ]) {
    it(`captures an actual setup caller in ${filename} (${source.startsWith('function') ? 'named' : 'anonymous'})`, () => {
      // Given
      const context = { site: buildOperationSite }
      // When
      const actual: unknown = runInNewContext(source, context, { filename })
      // Then
      assert(actual !== null && typeof actual === 'object')
      assert('file' in actual && 'operation' in actual)
      assert('line' in actual && typeof actual.line === 'number')
      assert('column' in actual && typeof actual.column === 'number')
      expect(actual.file).toBe(filename)
      expect(actual.operation).toBe('configure')
      expect(actual.line).toBeGreaterThan(0)
      expect(actual.column).toBeGreaterThan(0)
    })
  }
}

it.each([undefined, 'Error\n internal frame\n native-only frame'])(
  'refuses setup location fabrication when the stack formatter yields %s',
  (stack) => {
    // Given
    const prepare = Error.prepareStackTrace
    Error.prepareStackTrace = () => stack
    try {
      // When / Then
      expect(() => buildOperationSite('configure')).toThrow(
        'requires an actual source callsite',
      )
    } finally {
      Error.prepareStackTrace = prepare
    }
  },
)
