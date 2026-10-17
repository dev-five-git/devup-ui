import type { TestGroup } from './groups'
import { runTestGroups, type TestRun } from './run'

export async function prepareTests(
  group: TestGroup | undefined,
  run: TestRun,
): Promise<number | undefined> {
  switch (group) {
    case undefined:
      try {
        console.info(await runTestGroups(run))
        return 0
      } catch (error) {
        // This is the root CLI boundary: report once and prevent preload retries.
        console.error(error)
        return 1
      }
    case 'runtime': {
      const { register } = await import('../packages/bun-plugin/src/register')
      await register({ debug: true })
      return undefined
    }
    case 'isolated':
      return undefined
  }
}
