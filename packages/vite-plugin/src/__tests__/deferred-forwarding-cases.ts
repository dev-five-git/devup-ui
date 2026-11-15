import { expect, it } from 'bun:test'

import { fixture, obligation, root, target } from './deferred-erasure-fixture'

it.each([
  'current',
  'require',
  'cycle',
  'absent-export',
  'missing-edge',
  'ambiguous-stars',
  'star-default',
] as const)('verifies exact forwarding cuts when %s', async (kind) => {
  // Given current public resolver observations and final forwarding facts.
  const value = await fixture()
  value.begin()
  const forward = '/app/forward.js'
  const leaf = './leaf.mdx'
  const read = {
    ...obligation,
    request: './forward.js',
    exports: [kind === 'star-default' ? 'default' : 'Doc'],
    cutImporter: forward,
    cutRequest: leaf,
  }
  value.deferred.extracted(value.environment, root, JSON.stringify([read]))
  await value.edge({ specifier: read.request, resolved: forward })
  value.observeRoot('export {};', [forward])
  const code = {
    current: `export {default as Doc} from ${JSON.stringify(leaf)};`,
    require: `const Doc = require(${JSON.stringify(leaf)}).default; export {Doc};`,
    cycle: `export {Doc} from ${JSON.stringify(leaf)};`,
    'absent-export': 'export const Doc = 42;',
    'missing-edge': `export {default as Doc} from ${JSON.stringify(leaf)};`,
    'ambiguous-stars': 'export * from "./a.js"; export * from "./b.js";',
    'star-default': `export * from ${JSON.stringify(leaf)};`,
  }[kind]
  value.modules.set(forward, {
    code,
    importedIds: [kind === 'cycle' ? forward : target],
  })
  if (kind !== 'missing-edge')
    await value.edge({
      importer: forward,
      specifier: leaf,
      resolved: kind === 'cycle' ? forward : target,
    })
  value.observeTarget(true)
  value.deferred.resolution.succeeded(value.environment, target)
  // When the real verifier traverses those facts, Then only the unique supported cut succeeds.
  const verify = () =>
    value.deferred.verify(value.environment, value.graph(read))
  if (kind === 'current') expect(verify).not.toThrow()
  else expect(verify).toThrow(/page\.tsx:1:8:.*forwarding-unproven/)
})
