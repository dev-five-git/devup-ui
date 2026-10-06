import { createHash } from 'node:crypto'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  utimesSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterEach } from 'bun:test'

import { composeMdxRules, requireMdxPipeline } from '../mdx-pipeline'
import { createMdxSourceManager } from '../mdx-source-generation'
import type { MdxBuildBinding, MdxExtractionView } from '../mdx-source-types'
import { createAppContext } from '../session'
import {
  createWasm,
  extractWithModuleResolver,
  withModuleResolver,
} from '../wasm'
import {
  releaseSourceReporter,
  type ReporterOptions,
  sourceReporter,
} from './mdx-source-reporter'

const workspace = resolve(import.meta.dir, '../../../..')
const installedRoot = join(workspace, 'apps/landing')
const installed = createRequire(join(installedRoot, 'package.json'))
const roots: string[] = []
afterEach(() => {
  for (const root of roots.splice(0)) {
    releaseSourceReporter(root)
    rmSync(root, { recursive: true, force: true })
  }
})

export function sourceFixture(
  files: Record<string, string>,
  overrides: Partial<MdxBuildBinding> = {},
  development = false,
) {
  const root = mkdtempSync(join(tmpdir(), 'devup-ui-mdx-source-'))
  roots.push(root)
  symlinkSync(
    join(installedRoot, 'node_modules'),
    join(root, 'node_modules'),
    'junction',
  )
  function write(file: string, source: string, old = true) {
    const path = resolve(root, file)
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, source)
    if (old)
      utimesSync(path, new Date(1_700_000_000_000), new Date(1_700_000_000_000))
    return path
  }
  write('package.json', '{}')
  for (const [file, source] of Object.entries(files)) write(file, source)
  const reporter = sourceReporter(root)
  const reporterFile = resolve(import.meta.dir, 'mdx-source-reporter.ts')
  const reporterRequest = `require(${JSON.stringify(reporterFile)}).sourceReporter(${JSON.stringify(root)})`
  const plugin = write(
    'reporting-remark.cjs',
    `module.exports = ${reporterRequest}.remark`,
  )
  const raw = write(
    'reporting-raw.cjs',
    `module.exports = ${reporterRequest}.raw`,
  )
  const pluginOptions: ReporterOptions = {}
  const compilerOptions = {
    jsx: true,
    format: 'mdx',
    providerImportSource: 'provider',
    remarkPlugins: [[plugin, pluginOptions]],
  }
  const composed = composeMdxRules(
    {
      bundler: 'webpack',
      rules: [
        {
          use: [
            {
              loader: installed.resolve('@next/mdx/mdx-js-loader'),
              options: compilerOptions,
            },
            { loader: raw, options: { plugin } },
          ],
        },
      ],
      aliases: {},
    },
    { loader: 'devup' },
  )
  const pipeline = requireMdxPipeline(root, composed.pipelines[0])
  const cwd = process.cwd()
  const env = process.env.NODE_ENV
  process.chdir(root)
  process.env.NODE_ENV = development ? 'development' : 'production'
  const context = createAppContext(
    { pageExtensions: ['tsx', 'mdx', 'md'] },
    { singleCss: true },
  )
  process.chdir(cwd)
  process.env.NODE_ENV = env
  const aliases = {
    provider$: write(
      'provider.tsx',
      'export function useMDXComponents() { return {} }',
    ),
  }
  const owner = {}
  let extractionCalls = 0
  function configure(
    engine: ReturnType<typeof createWasm>,
    view: MdxExtractionView,
  ) {
    withModuleResolver(engine, root, view.resolver)
    engine.importCanonicalMap(view.plan.canonicalMap)
    engine.seedFileMap([...view.plan.seedFiles])
  }
  const binding: MdxBuildBinding = Object.freeze<MdxBuildBinding>({
    effectiveAppContext: context,
    extensions: ['.md', '.mdx'],
    aliases,
    conditions: ['import', 'node'],
    configFile: join(root, 'next.config.mjs'),
    async selectPipeline() {
      return {
        pipeline,
        context: {
          owner,
          generation: owner,
          compiler: owner,
          sourceMap: true,
          mode: development ? 'development' : 'production',
        },
        identity: pipeline.loaders.map((loader) => ({
          path: loader.loader,
          hash: createHash('sha256')
            .update(readFileSync(loader.loader))
            .digest('hex'),
          options: loader.options,
        })),
      }
    },
    ordinaryEligibility: () => ({ kind: 'disk-first' }),
    configureWasm: configure,
    async extractDependencies(view, signal) {
      extractionCalls += 1
      signal.throwIfAborted()
      const engine = createWasm(workspace)
      configure(engine, view)
      return view.inputs.map((input) => {
        const output = extractWithModuleResolver(engine, false, [
          input.filename,
          input.source,
          '@devup-ui/react',
          './df',
          true,
          false,
          false,
          {},
        ])
        try {
          return {
            filename: input.filename,
            dependencies: [...output.dependencies],
          }
        } finally {
          output.free()
        }
      })
    },
    ...overrides,
  })
  return {
    root,
    write,
    binding,
    pipeline,
    compilerOptions,
    pluginOptions,
    manager: createMdxSourceManager(binding),
    signal: new AbortController().signal,
    extractionCalls: () => extractionCalls,
    counts: reporter.counts,
    css(
      generation: Awaited<
        ReturnType<ReturnType<typeof createMdxSourceManager>['prepare']>
      >,
    ) {
      const engine = createWasm(workspace)
      generation.configureWasm(engine)
      for (const input of generation.inputs)
        extractWithModuleResolver(engine, false, [
          input.filename,
          input.source,
          '@devup-ui/react',
          './df',
          true,
          false,
          false,
          {},
        ]).free()
      return engine.getCss(null, false)
    },
  }
}

export const styledMdx = `import { Box, css } from '@devup-ui/react'

# Test

<Box bg="red" />
`
