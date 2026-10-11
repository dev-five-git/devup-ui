import { parseTestGroup } from './test-harness/groups'
import { prepareTests } from './test-harness/preload'
import { rootTestRun } from './test-harness/run'

export async function initialiseHarness(): Promise<void> {
  const status = await prepareTests(
    parseTestGroup(process.env.DEVUP_TEST_GROUP),
    rootTestRun(import.meta.dir),
  )
  if (status !== undefined) process.exit(status)
}

await initialiseHarness()
