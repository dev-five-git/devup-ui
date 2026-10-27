import { relative, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { nativeFixture } from '../../../../test-harness/native-fixture'
import {
  rootRequired,
  viteRequired,
} from '../../../../test-harness/source-inventory'

it.each(['root', 'vite'] as const)(
  'rejects an omitted actually loaded stable helper in the %s inventory',
  async (lane) => {
    // Given a genuine loaded corpus helper already observed in both full lanes.
    const repository = resolve(import.meta.dir, '../../../..')
    const helper =
      'packages/plugin-utils/src/__tests__/oracle-scanner-fixtures.ts'
    const fixture = nativeFixture({
      'source.ts': 'export function value(){return 42}\n',
      'source.test.ts': `import {it,expect} from 'bun:test';import {value} from './source';import {oracleScannerFixtures} from ${JSON.stringify(resolve(repository, helper))};it('actual',()=>{expect(value()).toBe(42);expect(oracleScannerFixtures.length).toBeGreaterThan(0)})`,
    })
    try {
      const result = await fixture.run()
      console.info(
        JSON.stringify({
          event: 'native-omission-control',
          lane,
          raw: result.lcov,
        }),
      )
      expect(result.code).toBe(0)
      const sources = result.sources.map((source) => ({
        ...source,
        file: relative(
          repository,
          resolve(fixture.root, source.file),
        ).replaceAll('\\', '/'),
      }))
      expect(sources.some((source) => source.file === helper)).toBe(true)
      const inventory =
        lane === 'root' ? rootRequired : viteRequired(repository)
      const required = inventory.filter((file) =>
        sources.some((source) => source.file === file),
      )
      expect(() => auditCoverage(sources, required)).not.toThrow()
      // When one whole stable record disappears, leaving the real complete sibling.
      const omitted = sources.filter((source) => source.file !== helper)
      // Then default ownership still requires that helper, independently per lane.
      expect(() => auditCoverage(omitted, required)).toThrow(
        `missing source ${helper}`,
      )
    } finally {
      fixture.dispose()
    }
  },
  120000,
)
