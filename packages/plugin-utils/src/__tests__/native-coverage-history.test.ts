import { expect, it } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { nativeFixture } from '../../../../test-harness/native-fixture'

it.each([true, false])(
  'keeps real preload branch history with complete=%s',
  async (complete) => {
    const fixture = nativeFixture(
      {
        'source.ts':
          'export function choice(active:boolean){\n if(active){\n return 1\n }\n return 2\n}\n',
        'early.ts': `import {choice} from './source';globalThis.earlyResult=[choice(true)];${complete ? 'globalThis.earlyResult.push(choice(false));' : ''}\n`,
        'source.test.ts': `import {it,expect} from 'bun:test';it('preload ran',()=>expect(globalThis.earlyResult).toEqual(${JSON.stringify(complete ? [1, 2] : [1])}))`,
      },
      ['./early.ts'],
    )
    try {
      const result = await fixture.run()
      expect(result.code).toBe(complete ? 0 : 1)
      expect(result.stderr).toMatch(/^\s*1 pass$/m)
      expect(result.stderr).toMatch(/^\s*0 fail$/m)
      if (complete)
        expect(() =>
          auditCoverage(result.sources, ['source.ts', 'early.ts']),
        ).not.toThrow()
      else {
        expect(() => auditCoverage(result.sources, ['source.ts'])).toThrow(
          'below 100%',
        )
        expect(
          result.sources
            .find((source) => source.file === 'source.ts')
            ?.lines.get(5),
        ).toBe(0)
      }
    } finally {
      fixture.dispose()
    }
  },
  120000,
)
