import { readFile, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'
import { build } from 'vite'

import { createCompiledGuard } from '../compiled-guard'
import { nativeWatcher } from './native-watcher'
import { nativeCompletions, productionFixture } from './production-fixture'

const devup = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
).resolve('@devup-ui/react')

it('retains parsing-phase forwarding authority after an earlier buildEnd clears it', async () => {
  // Given a resolver whose forwarding target exists only during parsing.
  const fixture = await productionFixture()
  const entry = await fixture.file(
    'app/src/main.custom',
    'import {Widget} from "./barrel.js";\nexport const value = Widget',
  )
  await fixture.file(
    'app/src/barrel.js',
    'export {Box as Widget} from "phase-provider"',
  )
  let available = true
  let captured = 0
  try {
    // When the actual native build finalizes after resolver state is released.
    const action = build({
      root: fixture.root,
      configFile: false,
      logLevel: 'silent',
      build: {
        write: false,
        lib: { entry, formats: ['es'] },
        rolldownOptions: { external: ['react', 'react/jsx-runtime'] },
      },
      plugins: [
        {
          name: 'phase-sensitive-provider',
          resolveId(id) {
            if (id === 'phase-provider' && available) {
              captured++
              return devup
            }
          },
          async load(id) {
            if (id === entry) return readFile(id, 'utf8')
          },
          buildEnd: {
            order: 'pre',
            handler() {
              available = false
            },
          },
        },
        createCompiledGuard({
          package: '@devup-ui/react',
          mdxExtensions: [],
          importAliases: {},
          extractCss: true,
        }),
      ],
    })
    // Then the real unsupported consumer retains its reference and location.
    await expect(action).rejects.toThrow(
      `${entry}:2:22: (in compiled output) untransformed module cannot use import './barrel.js' (Widget) at build time under extension '.custom'; add '.custom' to mdxExtensions`,
    )
    expect(available).toBe(false)
    expect(captured).toBeGreaterThan(1)
  } finally {
    await fixture.close()
  }
})

it('rejects a late unsupported watch consumer through an unchanged cached barrel and recovers', async () => {
  // Given a successful native watch generation with no diagnostic root.
  const fixture = await productionFixture()
  const initial = 'import "./barrel.js"; export const value=42'
  const entry = await fixture.file('app/src/main.js', initial)
  const barrel = await fixture.file(
    'app/src/barrel.js',
    'export {Box as Widget} from "@devup-ui/react"',
  )
  const consumer = await fixture.file(
    'app/src/late.custom',
    'import {Widget} from "./barrel.js";\nexport const value = Widget',
  )
  const parsed: string[] = []
  const result = await build({
    root: fixture.root,
    configFile: false,
    logLevel: 'silent',
    resolve: { alias: { '@devup-ui/react': devup } },
    plugins: [
      {
        name: 'native-watch-consumer-provider',
        async load(id) {
          if (id === consumer) return readFile(id, 'utf8')
        },
        moduleParsed(info) {
          parsed.push(info.id)
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
      watch: {},
      write: false,
      lib: { entry, formats: ['es'] },
      rolldownOptions: { external: ['react', 'react/jsx-runtime'] },
    },
  })
  const watcher = nativeWatcher(result)
  const next = nativeCompletions(watcher)
  try {
    await next()
    expect(parsed).toContain(barrel)
    parsed.length = 0
    // When an actual source change introduces an unsupported consumer.
    await writeFile(entry, 'export {value} from "./late.custom"')
    // Then the unchanged forwarding module still provides a located rejection.
    await expect(next()).rejects.toThrow(
      `${consumer}:2:22: (in compiled output) untransformed module cannot use import './barrel.js' (Widget) at build time under extension '.custom'; add '.custom' to mdxExtensions`,
    )
    await writeFile(barrel, 'export function Widget(){return 42}')
    await next()
    await writeFile(barrel, 'export {Box as Widget} from "@devup-ui/react"')
    await expect(next()).rejects.toThrow(`${consumer}:2:22:`)
    await writeFile(entry, initial)
    await next()
  } finally {
    await watcher.close()
    await fixture.close()
  }
}, 30000)
