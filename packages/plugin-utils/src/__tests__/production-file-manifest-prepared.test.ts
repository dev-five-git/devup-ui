import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { type PreparedSource } from '../import-graph'
import type { ProductionFileManifestOptions } from '../production-file-manifest'
import { enumerateProductionSourceFiles } from '../production-source-files'

async function collectProductionFileManifest(
  options: ProductionFileManifestOptions,
) {
  return (
    await import('../production-file-manifest')
  ).collectProductionFileManifest(options)
}

async function fixture(
  run: (
    root: string,
    file: (path: string, code?: string) => string,
  ) => Promise<void>,
) {
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-p2-prepared-')),
  )
  const file = (path: string, code = 'export {}') => {
    const target = join(root, path)
    mkdirSync(dirname(target), { recursive: true })
    writeFileSync(target, code)
    return target
  }
  try {
    await run(root, file)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

it('follows a real project-compiled provider when selected Markdown is prepared', async () => {
  await fixture(async (root, file) => {
    // Given the landing project's actual compiler and a provider outside selected roots.
    const project = createRequire(resolve('apps/landing/package.json'))
    const compiler = createRequire(project.resolve('@mdx-js/loader'))
    const {
      compile,
    }: {
      readonly compile: (
        input: { readonly path: string; readonly value: string },
        options: {
          readonly providerImportSource: string
          readonly jsx: boolean
          readonly SourceMapGenerator: unknown
        },
      ) => Promise<{ readonly value: string; readonly map?: unknown }>
    } = compiler('@mdx-js/mdx')
    const {
      SourceMapGenerator,
    }: {
      readonly SourceMapGenerator: unknown
    } = compiler('source-map')
    const markdown = '# Plain documentation\n\n<div>import "phantom"</div>'
    const entry = file('src/page.mdx', markdown)
    const provider = file('outside/provider.js', "export * from './leaf.js'")
    const leaf = file('outside/leaf.js')
    file('node_modules/phantom/package.json', '{')
    const prepared = await compile(
      { path: entry, value: markdown },
      { providerImportSource: 'provider', jsx: true, SourceMapGenerator },
    )
    const visits: string[] = []
    // When the existing PrepareSource hook supplies actual compiled JavaScript/JSX.
    const result = await collectProductionFileManifest({
      contexts: [
        {
          key: 'mdx',
          files: enumerateProductionSourceFiles({
            roots: ['src'],
            cwd: root,
            includeMdx: true,
          }),
          resolverOptions: { cwd: root, includeMdx: true, alias: { provider } },
          toId: (path) => path,
          prepareSource: (path) => {
            visits.push(path)
            return path === entry
              ? {
                  code: String(prepared),
                  map: prepared.map,
                  sourceType: 'compiled-mdx',
                }
              : undefined
          },
        },
      ],
    })
    // Then compiler-injected physical dependencies, not Markdown prose, close the inventory.
    expect(result.map(({ path }) => path)).toEqual(
      [entry, provider, leaf].sort(),
    )
    expect(visits.sort()).toEqual([entry, provider, leaf].sort())
    expect(prepared.map).toBeDefined()
  })
})

it.each([undefined, '', { code: '' }])(
  'uses raw code only when preparation is %j',
  async (prepared) => {
    await fixture(async (root, file) => {
      // Given a raw ordinary source import and a preparer with distinguishable empty output.
      const entry = file('src/main.ts', "import '../outside/leaf'")
      const leaf = file('outside/leaf.ts')
      // When existing prepared-source semantics determine the scanned code.
      const result = await collectProductionFileManifest({
        contexts: [
          {
            key: 'ordinary',
            files: enumerateProductionSourceFiles({
              roots: ['src'],
              cwd: root,
            }),
            resolverOptions: { cwd: root },
            toId: (path) => path,
            prepareSource: () => prepared,
          },
        ],
      })
      // Then empty prepared output cannot fall back to the raw import.
      expect(result.map(({ path }) => path)).toEqual(
        (prepared === undefined ? [entry, leaf] : [entry]).sort(),
      )
    })
  },
)

it.each([false, true])(
  'uses filename grammar when ordinary TS preparation exists=%s',
  async (prepared) => {
    await fixture(async (root, file) => {
      // Given a comparison expression whose import would disappear under unconditional JSX grammar.
      const code =
        'const less = a < b; import("../outside/leaf.js"); import type {T} from "phantom";'
      const entry = file('src/main.ts', code)
      const leaf = file('outside/leaf.js')
      file('node_modules/phantom/package.json', '{')
      // When ordinary TS is raw or prepared without compiled-mdx mode.
      const result = await collectProductionFileManifest({
        contexts: [
          {
            key: 'ts',
            files: enumerateProductionSourceFiles({
              roots: ['src'],
              cwd: root,
            }),
            resolverOptions: { cwd: root },
            toId: (path) => path,
            ...(prepared
              ? {
                  prepareSource: (path: string) =>
                    path === entry ? code : undefined,
                }
              : {}),
          },
        ],
      })
      // Then the literal edge and type-only elision follow the actual TS grammar.
      expect(result.map(({ path }) => path)).toEqual([entry, leaf].sort())
    })
  },
)

it.each(['ts', 'mdown'])(
  'uses compiled JSX grammar when physical filename ends in %s',
  async (extension) => {
    await fixture(async (root, file) => {
      // Given compiled JSX text, real brace imports, reexports and literal require.
      const entry = file(`src/page.${extension}`, '# Raw decoy')
      const leaves = ['dynamic', 'required', 'exported'].map((name) =>
        file(`outside/${name}.js`),
      )
      file('node_modules/phantom/package.json', '{')
      const prepared: PreparedSource = {
        code: 'const v=<div>import "phantom" {import("../outside/dynamic.js")}{require("../outside/required.js")}</div>; export * from "../outside/exported.js";',
        sourceType: 'compiled-mdx',
      }
      // When explicit sourceType overrides the physical extension's grammar.
      const result = await collectProductionFileManifest({
        contexts: [
          {
            key: 'jsx',
            files: enumerateProductionSourceFiles({
              roots: ['src'],
              cwd: root,
              includeMdx: ['.mdown'],
            }),
            resolverOptions: { cwd: root, includeMdx: ['.mdown'] },
            toId: (path) => path,
            prepareSource: (path) => (path === entry ? prepared : undefined),
          },
        ],
      })
      // Then only scanner-authoritative code expressions contribute dependencies.
      expect(result.map(({ path }) => path)).toEqual([entry, ...leaves].sort())
    })
  },
)
