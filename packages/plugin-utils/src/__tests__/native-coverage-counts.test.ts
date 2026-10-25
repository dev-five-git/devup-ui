import { expect, it } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { nativeFixture } from '../../../../test-harness/native-fixture'

it('diagnoses the uncalled sibling beside real nested async and generator results', async () => {
  const fixture = nativeFixture({
    'source.ts':
      'export function f(){return 1}\nexport function h(){return 2}\nexport function nested(){return ()=>3}\nexport async function asyncValue(){return 4}\nexport function* generator(){yield 5}\n',
    'source.test.ts':
      "import {it,expect} from 'bun:test';import {f,nested,asyncValue,generator} from './source';it('actual',async()=>{expect(f()).toBe(1);expect(nested()()).toBe(3);expect(await asyncValue()).toBe(4);expect(generator().next().value).toBe(5)})",
  })
  try {
    const result = await fixture.run()
    expect(result.code).toBe(1)
    expect(result.stderr).toMatch(/^\s*1 pass$/m)
    expect(result.stderr).toMatch(/^\s*0 fail$/m)
    const source = result.sources.find((source) => source.file === 'source.ts')
    expect(source).toBeDefined()
    expect(source?.hitFunctions).toBeGreaterThan(0)
    expect(source?.hitFunctions).toBeLessThan(source?.foundFunctions ?? 0)
    expect(source?.lines.get(2)).toBe(0)
    expect(() => auditCoverage(result.sources, ['source.ts'])).toThrow(
      'below 100%',
    )
  } finally {
    fixture.dispose()
  }
}, 120000)
