import { fileURLToPath } from 'node:url'

import { expect, it } from 'bun:test'

import {
  discoverTests,
  groupRules,
  parseTestGroup,
  partitionTests,
  TestGroupingError,
} from '../../../../test-harness/groups'
import { rootTestRun } from '../../../../test-harness/run'

const root = fileURLToPath(new URL('../../../../', import.meta.url))

it('assigns every actual test exactly once when the full root is discovered', () => {
  const files = discoverTests(root)
  const groups = rootTestRun(root).groups
  const assigned = [...groups.isolated, ...groups.runtime].sort()
  expect(assigned).toEqual(files)
  expect(new Set(assigned).size).toBe(files.length)
  expect(
    groups.runtime.every((file) =>
      /packages\/(bun-plugin|components|react|rsbuild-plugin|vite-plugin)\//.test(
        file,
      ),
    ),
  ).toBe(true)
  expect(
    groups.isolated.some((file) => file.endsWith('mixed-admission.test.ts')),
  ).toBe(true)
})

it.each([
  { files: ['packages/unknown/src/x.test.ts'], rules: groupRules },
  {
    files: [
      'packages/plugin-utils/src/x.test.ts',
      'packages/plugin-utils/src/x.test.ts',
    ],
    rules: groupRules,
  },
  {
    files: ['packages/components/src/x.test.ts'],
    rules: [
      ...groupRules,
      { directory: 'packages/components/src/', group: 'isolated' as const },
    ],
  },
  {
    files: ['packages/components/src/x.test.ts'],
    rules: [...groupRules, groupRules[1]].filter((rule) => rule !== undefined),
  },
])(
  'refuses omitted or contradictory discovery when grouping is $files',
  ({ files, rules }) => {
    expect(() => partitionTests(files, rules)).toThrow(TestGroupingError)
  },
)

it('sorts the explicit groups when discovery arrives in reverse order', () => {
  const files = [
    'packages/webpack-plugin/z.test.ts',
    'packages/webpack-plugin/a.test.ts',
  ]
  expect(partitionTests(files).isolated).toEqual([...files].reverse())
})

it.each([undefined, 'runtime', 'isolated'] as const)(
  'accepts the known preload role when it is %s',
  (group) => {
    expect(parseTestGroup(group)).toBe(group)
  },
)

it('fails closed when the child role is unknown', () => {
  expect(() => parseTestGroup('unknown')).toThrow(TestGroupingError)
})
