import { afterEach, expect, it } from 'bun:test'

import {
  captureCoordinatorState,
  CoordinatorStateError,
  readCoordinatorState,
  restoreCoordinatorState,
  writeCoordinatorState,
} from '../state'
import { createTestApp, removeTestApps } from './coordinator-app'
import { box } from './coordinator-prepared-fixture'

afterEach(removeTestApps)

it('roundtrips a nonempty completion projection with the extracted checkpoint', async () => {
  // Given real extracted CSS and a completion plan with a numbered bucket.
  const app = createTestApp()
  const wasm = app.engine()
  wasm
    .codeExtract(
      'provider.tsx',
      box('blue'),
      '@devup-ui/react',
      './df',
      false,
      false,
      true,
      {},
    )
    .free()
  const plan = {
    canonicalMap: { 'provider.tsx': 'route.tsx', 'global.tsx': '@global' },
    expectedBaseFiles: ['provider.tsx', 'global.tsx'],
  }
  const snapshot = captureCoordinatorState({
    wasm,
    optionsKey: 'completion-key',
    project: app.root,
    revision: 8,
    inputs: [],
    plan,
  })
  const path = app.write('checkpoint.json', '')
  // When the complete checkpoint crosses the disk read/write boundary.
  await writeCoordinatorState(path, snapshot)
  const restored = readCoordinatorState(path, 'completion-key')
  // Then both projection members and actual engine CSS survive validation.
  expect(restored?.plan).toEqual(plan)
  if (restored === undefined) throw new Error('missing persisted checkpoint')
  const fresh = app.engine()
  restoreCoordinatorState(fresh, restored)
  expect(fresh.getCss(0, false)).toContain('background:blue')
})

it.each(
  [
    null,
    [],
    'plan',
    {},
    { canonicalMap: {} },
    { expectedBaseFiles: [] },
    { canonicalMap: null, expectedBaseFiles: [] },
    { canonicalMap: [], expectedBaseFiles: [] },
    { canonicalMap: 'bucket', expectedBaseFiles: [] },
    { canonicalMap: { 'provider.tsx': 1 }, expectedBaseFiles: [] },
    {
      canonicalMap: { 'provider.tsx': 'route.tsx' },
      expectedBaseFiles: 'file',
    },
    { canonicalMap: { 'provider.tsx': 'route.tsx' }, expectedBaseFiles: [1] },
  ].map((plan) => ({ plan })),
)('rejects an invalid persisted completion projection: %j', ({ plan }) => {
  // Given otherwise valid persisted engine state with an untrusted projection.
  const app = createTestApp()
  const snapshot = captureCoordinatorState({
    wasm: app.engine(),
    optionsKey: 'completion-key',
    project: app.root,
    revision: 8,
    inputs: [],
  })
  const path = app.write(
    'checkpoint.json',
    JSON.stringify({ ...snapshot, plan }),
  )
  // When reading even for a different options key.
  const read = () => readCoordinatorState(path, 'different-key')
  // Then the whole corrupt checkpoint rejects rather than becoming a cold start.
  expect(read).toThrow(CoordinatorStateError)
})
