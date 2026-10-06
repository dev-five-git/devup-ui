import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'

import {
  buildStaticImportGraph,
  type CreateModuleResolverOptions,
  importGraphFailureOf,
} from '@devup-ui/plugin-utils'
import { afterEach, beforeEach, expect, it } from 'bun:test'
import type { Configuration } from 'webpack'

import {
  createWasm,
  extractWithModuleResolver,
  withModuleResolver,
} from '../wasm'
import { webpackResourceResolver } from '../webpack-resource-delivery'
import { mdxRule, withSelector } from './webpack-resource-fixture'

const aliasFixture: () => {
  readonly root: string
  readonly entry: string
  readonly file: (name: string, code?: string) => string
  readonly dispose: () => void
  readonly installed: (
    request: string,
    options: CreateModuleResolverOptions & {
      readonly mainFields?: readonly string[]
    },
  ) => string | false
} = createRequire(import.meta.url)(
  '../../../plugin-utils/src/__tests__/alias-array-fixture.ts',
).aliasFixture

let fixture: ReturnType<typeof aliasFixture>
beforeEach(() => {
  fixture = aliasFixture()
})
afterEach(() => fixture.dispose())

async function observe(
  config: Configuration,
  request = 'provider',
  disabled = false,
) {
  return withSelector(
    { context: fixture.root, ...config, module: { rules: [mdxRule()] } },
    (selector, compiler, binding) => {
      const settings = disabled
        ? webpackResourceResolver({
            ...binding,
            effectiveConfiguration: {
              ...binding.effectiveConfiguration,
              resolve: Object.defineProperty(
                { ...binding.effectiveConfiguration.resolve },
                'alias',
                { value: false, enumerable: true },
              ),
            },
          })
        : selector
      const native = fixture.installed(request, {
        alias: disabled ? false : (compiler.options.resolve.alias ?? {}),
        mainFields: ['module', 'main'],
      })
      const graph = buildStaticImportGraph('src', undefined, {
        cwd: fixture.root,
        alias: settings.aliases,
      })
      const engine = createWasm(resolve(import.meta.dir, '../../../..'))
      withModuleResolver(engine, fixture.root, { alias: settings.aliases })
      const output = extractWithModuleResolver(engine, false, [
        'src/main.ts',
        `import { css } from '@devup-ui/react'; import { value } from ${JSON.stringify(request)}; export const cls = css({color: value === undefined ? 'red' : value})`,
        '@devup-ui/react',
        './df',
        true,
        false,
        false,
        {},
      ])
      try {
        return {
          aliases: settings.aliases,
          native,
          outcome: graph.requests[0]?.outcome,
          css: engine.getCss(null, false),
        }
      } finally {
        output.free()
      }
    },
  )
}

it.each(['first', 'empty', 'self'])(
  'matches native duplicate descriptor selection when earlier entry is %s',
  async (kind) => {
    // Given
    fixture.file('src/main.ts', "import 'provider'")
    const blue = fixture.file('blue.js', "export const value = 'blue'")
    const wrong = fixture.file('wrong.js', "export const value = 'orange'")
    const alias = [
      {
        name: 'provider',
        alias:
          kind === 'first'
            ? [join(fixture.root, 'missing'), blue]
            : kind === 'empty'
              ? []
              : ['provider'],
      },
      { name: 'provider', alias: kind === 'first' ? wrong : blue },
    ]
    // When
    const result = await observe({ resolve: { alias } })
    // Then
    expect(result.native).toBe(blue)
    expect(result.outcome).toEqual({ kind: 'resolved', path: blue })
    expect(result.css).toContain('color:blue')
  },
)

it.each([
  { name: 'provider$', request: 'provider$/child', onlyModule: false },
  { name: 'provider', request: 'provider', onlyModule: true },
])(
  'matches native ignored descriptor semantics when request is $request',
  async ({ name, request, onlyModule }) => {
    // Given
    fixture.file('src/main.ts', `import ${JSON.stringify(request)}`)
    const alias = [{ name, alias: false as const, onlyModule }]
    // When
    const result = await observe({ resolve: { alias } }, request)
    // Then
    expect(result.native).toBe(false)
    expect(result.outcome).toEqual({ kind: 'ignored' })
    expect(result.css).toContain('color:red')
  },
)

it('matches native subpath fallback when an earlier descriptor is exact', async () => {
  // Given
  const request = 'provider/child'
  fixture.file('src/main.ts', `import '${request}'`)
  const blue = fixture.file('target/child.js', "export const value = 'blue'")
  const alias = [
    { name: 'provider', alias: false as const, onlyModule: true },
    { name: 'provider', alias: dirname(blue) },
  ]
  // When
  const result = await observe({ resolve: { alias } }, request)
  // Then
  expect(result.native).toBe(blue)
  expect(result.outcome).toEqual({ kind: 'resolved', path: blue })
  expect(result.css).toContain('color:blue')
})

it('preserves enhanced-resolve global false at the effective resolver seam', async () => {
  // Given
  fixture.file('src/main.ts', "import 'provider'")
  fixture.file('node_modules/provider/package.json', '{"main":"index.js"}')
  const blue = fixture.file(
    'node_modules/provider/index.js',
    "export const value = 'blue'",
  )
  // When
  const result = await observe({}, 'provider', true)
  // Then
  expect(result.native).toBe(blue)
  expect(result.aliases).toBe(false)
  expect(result.outcome).toEqual({ kind: 'resolved', path: blue })
  expect(result.css).toContain('color:blue')
})

it.each(['module', 'main'])(
  'matches native directory fields when selected entry is %s',
  async (field) => {
    // Given
    fixture.file('src/main.ts', "import 'provider'")
    fixture.file(
      'target/package.json',
      JSON.stringify({
        module: field === 'module' ? 'module.js' : 'absent.js',
        main: 'main.js',
        exports: './wrong.js',
      }),
    )
    const blue = fixture.file(
      `target/${field}.js`,
      "export const value = 'blue'",
    )
    fixture.file('target/index.js', "export const value = 'orange'")
    const alias = [{ name: 'provider', alias: join(fixture.root, 'target') }]
    // When
    const result = await observe({
      resolve: { alias, mainFields: ['module', 'main'] },
    })
    // Then
    expect(result.native).toBe(blue)
    expect(result.outcome).toEqual({ kind: 'resolved', path: blue })
    expect(result.css).toContain('color:blue')
  },
)

it('keeps native rewriting failure terminal when a later duplicate would resolve', async () => {
  // Given
  const entry = fixture.file('src/main.ts', "import 'provider'")
  const later = fixture.file('later.js', "export const value = 'blue'")
  const alias = [
    { name: 'provider', alias: join(fixture.root, 'missing') },
    { name: 'provider', alias: later },
  ]
  // When
  const failure = await withSelector(
    {
      context: fixture.root,
      resolve: { alias },
      module: { rules: [mdxRule()] },
    },
    (selector, compiler) => {
      expect(() =>
        fixture.installed('provider', {
          alias: compiler.options.resolve.alias ?? {},
        }),
      ).toThrow()
      try {
        buildStaticImportGraph('src', undefined, {
          cwd: fixture.root,
          alias: selector.aliases,
        })
      } catch (error) {
        if (error instanceof Error) return error
        throw error
      }
      throw new TypeError('Expected terminal alias failure')
    },
  )
  // Then
  expect(failure.name).toBe('ModuleAliasCandidatesError')
  expect(importGraphFailureOf(failure)?.[0]).toMatchObject({
    importer: entry,
    specifier: 'provider',
    outcome: { kind: 'error', error: failure },
  })
})
