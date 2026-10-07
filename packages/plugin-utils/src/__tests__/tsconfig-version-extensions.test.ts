import { createRequire } from 'node:module'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { createModuleResolver } from '../import-graph'
import { resolvePackage } from '../owned-module-resolution'
import { createPreparedFixture } from './prepared-graph-fixture'

const ts: {
  readonly version: string
  readonly sys: object
  nodeNextJsonConfigResolver(
    request: string,
    importer: string,
    host: object,
  ): {
    readonly resolvedModule?: { readonly resolvedFileName: string }
  }
} = createRequire(import.meta.url)('@typescript/typescript6')
let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

for (const request of ['preset', 'preset/entry'])
  it.each([
    ['js', true],
    ['jsx', true],
    ['mjs', true],
    ['cjs', true],
    ['ts', true],
    ['tsx', true],
    ['mts', true],
    ['cts', true],
    ['json', true],
    ['d.ts', true],
    ['d.mts', true],
    ['d.cts', true],
    ['mjsx', false],
    ['cjsx', false],
    ['mtsx', false],
    ['ctsx', false],
  ] as const)(
    `uses only installed TS recognized extension %s for ${request}`,
    (extension, recognized) => {
      // Given a mapped config whose content cannot overwrite the separate value.ts module.
      expect(ts.version).toBe('6.0.3')
      const importer = file(
        'tsconfig.json',
        JSON.stringify({ extends: request }),
      )
      file(
        'node_modules/preset/package.json',
        JSON.stringify({
          typesVersions: {
            '*': {
              tsconfig: [`blue.${extension}`],
              entry: [`blue.${extension}`],
            },
          },
        }),
      )
      const config = file(
        `node_modules/preset/blue.${extension}`,
        '{"compilerOptions":{"paths":{"color":["value.ts"]}}}',
      )
      const target = file(
        'node_modules/preset/value.ts',
        "export const color='blue'",
      )
      file('node_modules/preset/tsconfig.json', '{}')
      const selected = recognized ? config : undefined
      // When this selecting seam encounters an explicit substitution extension.
      const oracle = ts.nodeNextJsonConfigResolver(request, importer, ts.sys)
        .resolvedModule?.resolvedFileName
      // Then both match the independently enumerated extension policy, not each other alone.
      expect(oracle && resolve(oracle)).toBe(selected)
      expect(
        resolvePackage(request, importer, { purpose: 'tsconfig-extends' }),
      ).toBe(selected)
      if (recognized)
        expect(
          createModuleResolver({ cwd: root })('color', 'main.ts')?.path,
        ).toBe(target)
      else
        expect(() => createModuleResolver({ cwd: root })).toThrow(
          'Cannot resolve tsconfig extends',
        )
    },
  )
