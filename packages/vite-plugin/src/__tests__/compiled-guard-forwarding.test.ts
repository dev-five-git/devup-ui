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

it.each([
  ['named', 'export {Box as Widget} from "@devup-ui/react"', 'Widget', true],
  [
    'default',
    'export {Box as default} from "@devup-ui/react"',
    'default',
    true,
  ],
  [
    'import default',
    'import B from "./default.js"; export default B',
    'default',
    true,
  ],
  ['namespace', 'export * as DU from "@devup-ui/react"', 'DU.Box', true],
  ['star', 'export * from "@devup-ui/react"', 'Box', true],
  ['cycle', 'export * from "./cycle.js"', 'Box', true],
  [
    'duplicate origin',
    'export * from "./a.js"; export * from "./b.js"',
    'Box',
    true,
  ],
  [
    'local precedence',
    'export * from "@devup-ui/react"; export const Box=42',
    'Box',
    false,
  ],
  [
    'ambiguous',
    'export * from "./a.js"; export * from "./ordinary.js"',
    'Box',
    false,
  ],
  ['no star default', 'export * from "./default.js"', 'default', false],
] as const)(
  'retains native %s forwarding behavior',
  async (_name, code, member, rejects) => {
    // Given ordinary selected barrels between an unsupported root and public UI.
    const fixture = await productionFixture()
    const entry = await fixture.file(
      'app/src/main.custom',
      `import * as forwarded from "./barrel.js";\nexport const value = forwarded.${member}`,
    )
    await fixture.file('app/src/barrel.js', code)
    await fixture.file('app/src/a.js', 'export {Box} from "@devup-ui/react"')
    await fixture.file('app/src/b.js', 'export {Box} from "@devup-ui/react"')
    await fixture.file('app/src/ordinary.js', 'export const Box=42')
    await fixture.file(
      'app/src/default.js',
      'export {Box as default} from "@devup-ui/react"',
    )
    await fixture.file(
      'app/src/cycle.js',
      'export * from "./barrel.js"; export * from "@devup-ui/react"',
    )
    try {
      // When native Vite resolves the complete forwarding graph.
      const action = build({
        root: fixture.root,
        configFile: false,
        logLevel: 'silent',
        resolve: { alias: { '@devup-ui/react': devup } },
        plugins: [
          {
            name: 'native-forwarding-source',
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
      // Then only an unambiguous compile-time origin produces the located error.
      if (rejects)
        await expect(action).rejects.toThrow(
          `${entry}:2:22: (in compiled output) untransformed module cannot use import './barrel.js' (${member})`,
        )
      else expect(await action).toBeDefined()
    } finally {
      await fixture.close()
    }
  },
)
