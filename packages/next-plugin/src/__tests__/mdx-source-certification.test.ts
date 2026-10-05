import { join } from 'node:path'

import { createModuleResolver } from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'

import { immutableGeneration } from '../coordinator-generation'
import { createMdxSourceManager } from '../mdx-source-generation'
import { MdxNativeInputPendingError } from '../mdx-source-pending'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it('blocks finalization when extraction reports a new native-required input without reading it', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'df/native.ts': 'invalid until the upstream loader runs',
  })
  const filename = join(f.root, 'df/native.ts')
  const expectation = { filename, reason: 'upstream-loader', proof: {} }
  const manager = createMdxSourceManager({
    ...f.binding,
    ordinaryEligibility: (path) =>
      path === filename
        ? { kind: 'native-required', expectation }
        : { kind: 'disk-first' },
    async extractDependencies(view, signal) {
      const reports = await f.binding.extractDependencies(view, signal)
      return reports.map((report) =>
        report.filename === 'provider.tsx'
          ? { ...report, dependencies: [filename] }
          : report,
      )
    },
  })
  // When
  const generation = await manager.prepare(f.signal)
  // Then
  expect(() =>
    manager.validateForCssFinalization(immutableGeneration(generation)),
  ).toThrow(MdxNativeInputPendingError)
  expect(generation.pendingOrdinary).toEqual([expectation])
  expect(generation.watchInputs).toContain(filename)
  expect(
    generation.ordinaryInputs.some((input) => input.resourcePath === filename),
  ).toBe(false)
  expect(Object.isFrozen(generation.pendingOrdinary)).toBe(true)
})

it('blocks finalization when the published resolver discovers a later native-required input', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': styledMdx,
    'df/native.ts': 'invalid until the upstream loader runs',
  })
  const filename = join(f.root, 'df/native.ts')
  const manager = createMdxSourceManager({
    ...f.binding,
    ordinaryEligibility: (path) =>
      path === filename
        ? {
            kind: 'native-required',
            expectation: { filename, reason: 'upstream-loader', proof: {} },
          }
        : { kind: 'disk-first' },
  })
  const generation = await manager.prepare(f.signal)
  const supplied = immutableGeneration(generation)
  const snapshot = generation.pendingOrdinary
  const resolver = createModuleResolver({ cwd: f.root, ...generation.resolver })
  expect(() => resolver('../df/native', 'app/page.mdx')).toThrow(filename)
  // When / Then
  expect(() => manager.validateForCssFinalization(supplied)).toThrow(
    MdxNativeInputPendingError,
  )
  expect(generation.pendingOrdinary).toBe(snapshot)
  expect(snapshot).toEqual([])
  expect(Object.isFrozen(snapshot)).toBe(true)
})
