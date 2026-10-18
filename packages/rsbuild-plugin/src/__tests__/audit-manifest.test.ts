import { expect, it } from 'bun:test'

import { fixture, red, run } from './audit-fixture'

it('builds valid reachable input when an unused physical import has a null manifest', () => {
  // Given
  using fs = fixture()
  fs.put('devup.json', '{}')
  fs.put('src/entry.js', red)
  const control = run(fs.root, 'cold')
  expect(control.ok).toBe(true)
  fs.put('src/Unused.js', 'import "@devup-ui/broken";')
  fs.put('node_modules/@devup-ui/broken/package.json', 'null')
  // When
  const unused = run(fs.root, 'unused')
  // Then
  expect(unused.ok).toBe(true)
}, 120000)

it('retains a normal reachable malformed-package native error without a prepass', () => {
  // Given
  using fs = fixture()
  fs.put('src/entry.js', 'import "@devup-ui/broken";')
  fs.put('node_modules/@devup-ui/broken/package.json', 'null')
  // When
  const reachable = run(fs.root, 'reachable-native')
  // Then
  expect(reachable.ok).toBe(false)
  expect(typeof reachable.error).toBe('string')
}, 60000)
