import { symlinkSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { extractRequest } from '../coordinator-engine'
import { composeMdxRules, requireMdxPipeline } from '../mdx-pipeline'
import { compileMdx, createMdxDeadline } from '../mdx-prepare'
import { runPrewarm } from '../prewarm-run'
import { createAppContext } from '../session'
import { createWasm, withModuleResolver } from '../wasm'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()
const installedRoot = resolve(import.meta.dir, '../../../../apps/landing')
const installed = createRequire(join(installedRoot, 'package.json'))

it.each([
  { sourceMap: true, surface: 'request' },
  { sourceMap: false, surface: 'request' },
  { sourceMap: true, surface: 'prewarm' },
  { sourceMap: false, surface: 'prewarm' },
] as const)(
  'remaps project compiler maps through $surface with maps=$sourceMap',
  async ({ sourceMap, surface }) => {
    // Given actual @next/mdx compilation with its original source map.
    const filename = 'src/page.mdx'
    const root = makeProject({
      [filename]: `# Intro\n\n# More\n\n# Next\n\nimport { css } from '@devup-ui/react'\n\nexport const c = css({ bg: Math.random() })\n`,
    })
    symlinkSync(
      join(installedRoot, 'node_modules/next'),
      join(root, 'node_modules/next'),
      'junction',
    )
    const resourcePath = join(root, filename)
    const composition = composeMdxRules(
      {
        bundler: 'webpack',
        rules: [
          {
            test: /\.mdx$/,
            use: [
              {
                loader: installed.resolve('@next/mdx/mdx-js-loader'),
                options: { jsx: true },
              },
            ],
          },
        ],
        aliases: {},
      },
      { loader: 'devup' },
    )
    const compiled = await compileMdx({
      root,
      filename: resourcePath,
      pipeline: requireMdxPipeline(resourcePath, composition.pipelines[0]),
      signal: new AbortController().signal,
      deadline: createMdxDeadline(),
      context: { owner: {}, generation: {}, sourceMap: true },
    })
    process.chdir(root)
    const context = { ...createAppContext({}, {}), sourceMap }
    const engine = createWasm(root)
    const prepared = { code: compiled.source, map: compiled.map }
    withModuleResolver(engine, root, { prepareSource: () => prepared })
    // When both production extraction surfaces encounter the real WASM error.
    const request = () =>
      extractRequest(
        engine,
        {
          package: context.libPackage,
          cssDir: context.cssDir,
          singleCss: context.singleCss,
          sourceMap,
          importAliases: {},
        },
        { filename, resourcePath, code: compiled.source },
      )
    const prewarm = () =>
      runPrewarm({
        context,
        engine,
        files: [filename],
        collectMs: undefined,
        preparedInputs: new Map([[filename, prepared]]),
      })
    // Then real compiler coordinates point to original line 9, not emitted JS.
    expect({ request, prewarm }[surface]).toThrow(/page\.mdx:9:\d+: /)
  },
)
