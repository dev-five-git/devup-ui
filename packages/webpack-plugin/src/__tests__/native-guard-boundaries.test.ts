import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'
import type { Compiler, Configuration, Stats } from 'webpack'

import { registerCompiledGuard } from '../compiled-guard'
import { mdxBuildFixture, mdxLanding } from './mdx-build-fixture'

const bundled: { webpack(config: Configuration): Compiler } = mdxLanding(
  'next/dist/compiled/webpack/webpack',
)

async function compile(options: {
  readonly source: string
  readonly files?: Readonly<Record<string, string>>
  readonly plugin?: { apply(compiler: Compiler): void }
  readonly parserType?: string
}) {
  const fixture = mdxBuildFixture(options.source, '.opaque')
  for (const [name, code] of Object.entries(options.files ?? {}))
    writeFileSync(join(fixture.root, name), code)
  const compiler = bundled.webpack({
    context: fixture.root,
    mode: 'development',
    entry: fixture.filename,
    output: { path: join(fixture.root, 'out') },
    resolve: { alias: fixture.alias },
    optimization: { minimize: false },
    module: {
      rules: [
        { test: /\.opaque$/, type: options.parserType ?? 'javascript/auto' },
      ],
    },
    plugins: [
      ...(options.plugin ? [options.plugin] : []),
      {
        apply(compiler) {
          registerCompiledGuard(compiler, {
            package: '@devup-ui/react',
            mdxExtensions: ['.mdx'],
            importAliases: {},
          })
        },
      },
    ],
  })
  try {
    const stats = await new Promise<Stats>((resolve, reject) =>
      compiler.run((error, stats) => {
        if (error) reject(error)
        else if (stats) resolve(stats)
        else reject(new Error('Missing native compiler stats'))
      }),
    )
    return stats.toJson({ all: false, errors: true }).errors ?? []
  } finally {
    await new Promise<void>((resolve, reject) =>
      compiler.close((error) => (error ? reject(error) : resolve())),
    )
    fixture.cleanup()
  }
}

it('retains native compiler rejection for a public parser without JavaScript hooks', async () => {
  // Given: a public parser for a language with no JavaScript dependencies.
  const plugin: { apply(compiler: Compiler): void } = mdxLanding(
    join(import.meta.dir, 'native-language-plugin.cjs'),
  )
  // When: the actual compiler creates and runs that parser.
  const errors = await compile({
    source: 'opaque language payload',
    plugin,
  })
  // Then: Webpack retains its native parser-plugin incompatibility error.
  expect(errors).toHaveLength(1)
  expect(errors[0]?.message).toContain('TypeError:')
  expect(errors[0]?.message).toContain('hooks.new.for')
})

it('builds opaque language input through its own public parser and native generator', async () => {
  // Given: the same parser registered under its own language, not JavaScript.
  const plugin: { apply(compiler: Compiler): void } = mdxLanding(
    join(import.meta.dir, 'native-language-plugin.cjs'),
  )
  // When: the native compiler parses an opaque dependency-free statement.
  const errors = await compile({
    source: 'console.log("opaque");',
    parserType: 'opaque-language',
    plugin,
  })
  // Then: its parser and native generator complete without JavaScript hook errors.
  expect(errors).toEqual([])
})

it('allows a runtime-owned require when a public parser plugin handles it', async () => {
  // Given: require is supplied at runtime, not resolved into the bundle.
  // When: a genuine public parser hook claims the call before Webpack adds edges.
  const errors = await compile({
    source: 'const DU=require("runtime-owner"); export const value=DU.css;',
    plugin: {
      apply(compiler) {
        compiler.hooks.normalModuleFactory.tap('RuntimeRequire', (factory) => {
          factory.hooks.parser
            .for('javascript/auto')
            .tap('RuntimeRequire', (parser) => {
              parser.hooks.call
                .for('require')
                .tap({ name: 'RuntimeRequire', stage: -10000 }, () => true)
            })
        })
      },
    },
  })
  // Then: parser facts without resolved graph dependencies stay opaque.
  expect(errors).toEqual([])
})

it.each([
  ['renamed', 'export {Box as Widget} from "@devup-ui/react"', 'Widget'],
  ['namespace', 'export * as DU from "@devup-ui/react"', 'DU.Box'],
])(
  'rejects compile-time uses through native %s forwarding',
  async (_name, forward, use) => {
    // Given: actual reexports and a named use outside selected extensions.
    // When: Webpack builds the real module graph.
    const errors = await compile({
      source: `import * as forwarded from "./forward.js"; console.log(forwarded.${use});`,
      files: { 'forward.js': forward },
      plugin: {
        apply(compiler) {
          compiler.hooks.thisCompilation.tap(
            'EarlyNativeExports',
            (compilation) => {
              compilation.hooks.finishModules.intercept({
                register(tap) {
                  return tap.name === 'FlagDependencyExportsPlugin'
                    ? { ...tap, stage: -10000 }
                    : tap
                },
              })
            },
          )
        },
      },
    })
    // Then: the compiler reports the located guard error.
    expect(
      errors.some((error) => error.message.includes('mdxExtensions')),
    ).toBe(true)
  },
)
