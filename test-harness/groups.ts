import { readdirSync } from 'node:fs'
import { join } from 'node:path'

export type TestGroup = 'isolated' | 'runtime'
export interface GroupRule {
  readonly directory: string
  readonly group: TestGroup
}

export const groupRules: readonly GroupRule[] = [
  { directory: 'packages/bun-plugin/', group: 'runtime' },
  { directory: 'packages/components/', group: 'runtime' },
  { directory: 'packages/react/', group: 'runtime' },
  { directory: 'packages/eslint-plugin/', group: 'isolated' },
  { directory: 'packages/next-plugin/', group: 'isolated' },
  { directory: 'packages/plugin-utils/', group: 'isolated' },
  { directory: 'packages/reset-css/', group: 'isolated' },
  { directory: 'packages/rsbuild-plugin/', group: 'runtime' },
  { directory: 'packages/vite-plugin/', group: 'runtime' },
  { directory: 'packages/webpack-plugin/', group: 'isolated' },
]

export class TestGroupingError extends Error {
  constructor(readonly detail: string) {
    super(`Test grouping: ${detail}`)
    this.name = 'TestGroupingError'
  }
}

export function discoverTests(root: string, directory = 'packages'): string[] {
  return readdirSync(join(root, directory), { withFileTypes: true })
    .flatMap((entry) => {
      const path = `${directory}/${entry.name}`
      const name = entry.name.toLowerCase()
      if (entry.isDirectory()) {
        return name === 'node_modules' || name.startsWith('.')
          ? []
          : discoverTests(root, path)
      }
      return /(?:\.|_)(?:test|spec)\.(?:js|jsx|ts|tsx|mjs|cjs|mts|cts)$/.test(
        name,
      )
        ? [path]
        : []
    })
    .sort()
}

export function partitionTests(
  files: readonly string[],
  rules: readonly GroupRule[] = groupRules,
): Record<TestGroup, string[]> {
  const result: Record<TestGroup, string[]> = { isolated: [], runtime: [] }
  const seen = new Set<string>()
  for (const file of files) {
    const matches = rules.filter((rule) => file.startsWith(rule.directory))
    if (seen.has(file) || matches.length !== 1) {
      throw new TestGroupingError(`${file} must belong to exactly one group`)
    }
    seen.add(file)
    for (const rule of matches) result[rule.group].push(file)
  }
  for (const group of Object.values(result)) group.sort()
  return result
}

export function parseTestGroup(
  value: string | undefined,
): TestGroup | undefined {
  switch (value) {
    case undefined:
    case 'isolated':
    case 'runtime':
      return value
    default:
      throw new TestGroupingError(`unknown child group ${value}`)
  }
}
