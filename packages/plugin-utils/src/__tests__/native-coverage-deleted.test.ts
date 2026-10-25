import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { nativeFixture } from '../../../../test-harness/native-fixture'

it.each(['cjs', 'mjs'] as const)(
  'retains deleted %s caller loader and plugin after real teardown and GC',
  async (extension) => {
    const modules = mkdtempSync(join(tmpdir(), 'devup-native-deleted-'))
    const inputs =
      extension === 'cjs'
        ? {
            'plugin.cjs':
              'exports.value = function value(input) { return input + 1 }\n',
            'loader.cjs':
              "const plugin=require('./plugin.cjs');exports.load=function load(input){return plugin.value(input)}\n",
            'caller.cjs':
              "const loader=require('./loader.cjs');exports.modifyConfig=function modifyConfig(input){return loader.load(input)}\n",
          }
        : {
            'plugin.mjs': 'export function value(input) { return input + 1 }\n',
            'loader.mjs':
              "import {value} from './plugin.mjs';export function load(input){return value(input)}\n",
            'caller.mjs':
              "import {load} from './loader.mjs';export function modifyConfig(input){return load(input)}\n",
          }
    const files = Object.entries(inputs).map(([name, source]) => ({
      path: join(modules, name),
      source,
    }))
    const caller = JSON.stringify(join(modules, `caller.${extension}`))
    const load =
      extension === 'cjs'
        ? `createRequire(import.meta.url)(${caller})`
        : `await import(${caller}+'?first')`
    const reload =
      extension === 'mjs'
        ? `expect((await import(${caller}+'?last')).modifyConfig(41)).toBe(42);`
        : ''
    const fixture = nativeFixture({
      'source.test.ts': `import {it,expect} from 'bun:test';import {writeFileSync,rmSync} from 'node:fs';import {createRequire} from 'node:module';it('write load delete',async()=>{for(const input of ${JSON.stringify(files)})writeFileSync(input.path,input.source);try{const caller=${load};expect(caller.modifyConfig(41)).toBe(42);${reload}}finally{rmSync(${JSON.stringify(modules)},{recursive:true});Bun.gc(true)}})`,
    })
    try {
      const result = await fixture.run()
      expect(result.code).toBe(0)
      const witnesses = result.sources.filter((source) =>
        files.some((file) => resolve(fixture.root, source.file) === file.path),
      )
      expect(witnesses).toHaveLength(3)
      expect(() =>
        auditCoverage(
          witnesses,
          witnesses.map((source) => source.file),
        ),
      ).not.toThrow()
    } finally {
      fixture.dispose()
      rmSync(modules, { recursive: true, force: true })
    }
  },
  120000,
)

it('rejects an uncalled function even after its module is deleted', async () => {
  const fixture = nativeFixture({
    'source.ts':
      'export function value(){return 42}\nexport function unused(){return 99}\n',
    'source.test.ts':
      "import {it,expect} from 'bun:test';import {rmSync} from 'node:fs';import {value} from './source';it('delete',()=>{expect(value()).toBe(42);rmSync(new URL('./source.ts',import.meta.url));Bun.gc(true)})",
  })
  try {
    const result = await fixture.run()
    expect(result.code).toBe(1)
    expect(result.stderr).toMatch(/^\s*1 pass$/m)
    expect(result.stderr).toMatch(/^\s*0 fail$/m)
    expect(
      result.sources.find((source) => source.file === 'source.ts'),
    ).toMatchObject({ foundFunctions: 2, hitFunctions: 1 })
    expect(() => auditCoverage(result.sources, ['source.ts'])).toThrow(
      'below 100%',
    )
  } finally {
    fixture.dispose()
  }
}, 120000)
