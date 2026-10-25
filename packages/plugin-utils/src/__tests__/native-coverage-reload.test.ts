import { expect, it } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { nativeFixture } from '../../../../test-harness/native-fixture'

it('retains actual client server client query reloads and equivalent query functions after GC', async () => {
  const fixture = nativeFixture({
    'source.ts':
      "export const value=process.env.RELOAD_SIDE==='server'?'server':'client'\n",
    'value.ts': 'export function value(){return 1}\n',
    'source.test.ts':
      "import {it,expect} from 'bun:test';it('reloads',async()=>{process.env.RELOAD_SIDE='client';expect((await import('./source?first')).value).toBe('client');process.env.RELOAD_SIDE='server';expect((await import('./source?middle')).value).toBe('server');process.env.RELOAD_SIDE='client';expect((await import('./source?last')).value).toBe('client');expect((await import('./value?first')).value()).toBe(1);expect((await import('./value?last')).value()).toBe(1);Bun.gc(true)})",
  })
  try {
    const result = await fixture.run()
    expect(result.code).toBe(0)
    expect(() =>
      auditCoverage(result.sources, ['source.ts', 'value.ts']),
    ).not.toThrow()
    expect(
      result.sources.find((source) => source.file === 'value.ts')?.hitFunctions,
    ).toBe(1)
  } finally {
    fixture.dispose()
  }
}, 120000)
