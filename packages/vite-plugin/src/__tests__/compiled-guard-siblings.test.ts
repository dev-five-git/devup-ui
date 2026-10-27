import { readFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'
import { createBuilder } from 'vite'

import { createCompiledGuard } from '../compiled-guard'
import { productionFixture } from './production-fixture'

it('keeps sibling forwarding authority separate for the same native module IDs', async () => {
  // Given two real environments with different authorities for the same barrel.
  const fixture = await productionFixture()
  const entry = await fixture.file(
    'app/src/main.custom',
    'import {Widget} from "./barrel.js";\nexport const value = Widget',
  )
  const barrel = await fixture.file('app/src/barrel.js', '')
  const devup = createRequire(
    resolve(import.meta.dir, '../../../../apps/landing/package.json'),
  ).resolve('@devup-ui/react')
  const guard = createCompiledGuard({
    package: '@devup-ui/react',
    mdxExtensions: [],
    importAliases: {},
    extractCss: true,
  })
  try {
    const builder = await createBuilder({
      root: fixture.root,
      configFile: false,
      logLevel: 'silent',
      resolve: { alias: { '@devup-ui/react': devup } },
      plugins: [
        {
          name: 'sibling-forwarding-provider',
          sharedDuringBuild: true,
          async load(id) {
            if (id === entry) return readFile(id, 'utf8')
            if (id === barrel)
              return this.environment.name === 'client'
                ? 'export function Widget(){return 42}'
                : 'export {Box as Widget} from "@devup-ui/react"'
          },
        },
        guard,
      ],
      environments: {
        client: { build: { lib: { entry, formats: ['es'] } } },
        sibling: {
          consumer: 'client',
          build: { lib: { entry, formats: ['es'] } },
        },
      },
      build: {
        write: false,
        lib: { entry, formats: ['es'] },
        rolldownOptions: { external: ['react', 'react/jsx-runtime'] },
      },
      builder: {
        async buildApp(builder) {
          const { client, sibling } = builder.environments
          if (!client || !sibling)
            throw new TypeError('Expected native siblings')
          // When public native builds use one shared guard with overlapping graphs.
          const [runtime, compileTime] = await Promise.allSettled([
            builder.build(client),
            builder.build(sibling),
          ])
          // Then only the compile-time sibling rejects with the actual location.
          expect(runtime.status).toBe('fulfilled')
          expect(compileTime.status).toBe('rejected')
          if (compileTime.status !== 'rejected')
            throw new TypeError('Expected unsupported sibling rejection')
          const reason: unknown = compileTime.reason
          if (!(reason instanceof Error)) throw reason
          expect(reason.message).toContain(`${entry}:2:22:`)
          expect(reason.message).toContain("'./barrel.js' (Widget)")
          expect(reason.message).toContain('mdxExtensions')
        },
      },
    })
    await builder.buildApp()
  } finally {
    await fixture.close()
  }
})
