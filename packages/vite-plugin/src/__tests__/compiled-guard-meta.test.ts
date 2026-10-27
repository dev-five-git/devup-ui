import { readFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'
import { build } from 'vite'

import { createCompiledGuard } from '../compiled-guard'
import { productionFixture } from './production-fixture'

const devup = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
).resolve('@devup-ui/react')

const cases = [
  ['meta', 'export const value=import.meta;'],
  ['meta', 'export function value(){return import.meta}'],
  ['meta', 'export function value(meta){return import.meta}'],
  ['meta', '{let meta; globalThis.value=import.meta;}'],
  ['target', 'export function value(){return new.target}'],
  ['target', 'export function value(target){return new.target}'],
] as const

it.each(
  cases.flatMap(([name, body]) =>
    [false, true].flatMap((after) =>
      [false, true].map((read) => ({ name, body, after, read })),
    ),
  ),
)(
  'distinguishes native meta properties from lexical reads: %j',
  async ({ name, body, after, read }) => {
    // Given an unsupported final-JS module and an actual css import with the same suffix.
    const fixture = await productionFixture()
    const declaration = `import {css as ${name}} from '@devup-ui/react';`
    const source = `${after ? body + declaration : declaration + body}\n${read ? `void ${name};` : ''}`
    const entry = await fixture.file('app/src/main.custom', source)
    try {
      // When the real native Vite guard sees the final parsed module.
      const action = build({
        root: fixture.root,
        configFile: false,
        logLevel: 'silent',
        resolve: { alias: { '@devup-ui/react': devup } },
        plugins: [
          {
            name: 'native-meta-source',
            async load(id) {
              if (id === entry) return readFile(id, 'utf8')
            },
          },
          createCompiledGuard({
            package: '@devup-ui/react',
            mdxExtensions: [],
            importAliases: {},
            extractCss: true,
          }),
        ],
        build: {
          write: false,
          lib: { entry, formats: ['es'] },
          rolldownOptions: { external: ['react', 'react/jsx-runtime'] },
        },
      })
      // Then only the real binding read rejects, at that read rather than the meta suffix.
      if (read)
        await expect(action).rejects.toThrow(
          `${entry}:2:6: (in compiled output) untransformed module cannot use import '@devup-ui/react' (css)`,
        )
      else expect(await action).toBeDefined()
    } finally {
      await fixture.close()
    }
  },
)
