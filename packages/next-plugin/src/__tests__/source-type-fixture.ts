import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  utimesSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterEach } from 'bun:test'

import { extractInput } from '../coordinator-engine'
import { composeMdxRules, requireMdxPipeline } from '../mdx-pipeline'
import { createMdxSourceManager } from '../mdx-source-generation'
import type { MdxBuildBinding, MdxExtractionView } from '../mdx-source-types'
import { createAppContext } from '../session'
import { createWasm, withModuleResolver } from '../wasm'

const workspace = resolve(import.meta.dir, '../../../..')
const installedRoot = join(workspace, 'apps/landing')
const roots: string[] = []
afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})

export function sourceFixture(
  files: Readonly<Record<string, string>>,
  overrides: Partial<MdxBuildBinding> = {},
) {
  const root = mkdtempSync(join(tmpdir(), 'devup-source-type-'))
  roots.push(root)
  symlinkSync(
    join(installedRoot, 'node_modules'),
    join(root, 'node_modules'),
    'junction',
  )
  function write(file: string, code: string) {
    const path = join(root, file)
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, code)
    utimesSync(path, new Date(1_700_000_000_000), new Date(1_700_000_000_000))
    return path
  }
  write('package.json', '{}')
  for (const [file, code] of Object.entries(files)) write(file, code)
  const provider = write(
    'provider.jsx',
    'export function useMDXComponents() {return {}}',
  )
  const counter = write(
    'counter.cjs',
    'function loader(source) {loader.count++; return source}; loader.count=0; module.exports=loader',
  )
  const require = createRequire(join(root, 'package.json'))
  const compilerOptions = {
    jsx: true,
    format: 'mdx',
    providerImportSource: 'provider',
  }
  const pipeline = requireMdxPipeline(
    root,
    composeMdxRules(
      {
        bundler: 'webpack',
        rules: [
          {
            use: [
              {
                loader: require.resolve('@next/mdx/mdx-js-loader'),
                options: compilerOptions,
              },
              counter,
            ],
          },
        ],
        aliases: {},
      },
      { loader: 'devup' },
    ).pipelines[0],
  )
  const context = {
    ...createAppContext(
      { pageExtensions: ['mdx', 'md', 'tsx', 'jsx'] },
      { singleCss: true },
      root,
    ),
    phase: 'production' as const,
    watch: false,
  }
  const settings = {
    package: context.libPackage,
    cssDir: context.cssDir,
    singleCss: true,
    sourceMap: false,
    importAliases: {},
  }
  const owner = {}
  function configure(
    engine: ReturnType<typeof createWasm>,
    view: MdxExtractionView,
  ) {
    withModuleResolver(engine, root, view.resolver)
    engine.importCanonicalMap(view.plan.canonicalMap)
    engine.seedFileMap([...view.plan.seedFiles])
  }
  const binding: MdxBuildBinding = {
    effectiveAppContext: context,
    extensions: ['.md', '.mdx'],
    aliases: { provider$: provider },
    conditions: ['import', 'node'],
    configFile: join(root, 'next.config.mjs'),
    selectPipeline: async () => ({
      pipeline,
      context: { owner, generation: owner, sourceMap: true },
      identity: pipeline.loaders,
    }),
    ordinaryEligibility: () => ({ kind: 'disk-first' }),
    configureWasm: configure,
    async extractDependencies(view, signal) {
      signal.throwIfAborted()
      const engine = createWasm(root)
      configure(engine, view)
      return view.inputs.map((input) => ({
        filename: input.filename,
        dependencies: extractInput(engine, settings, input).dependencies ?? [],
      }))
    },
    ...overrides,
  }
  return {
    root,
    write,
    binding,
    compilerOptions,
    manager: createMdxSourceManager(binding),
    signal: new AbortController().signal,
    counts() {
      const module: unknown = require(counter)
      if (
        typeof module !== 'function' ||
        !('count' in module) ||
        typeof module.count !== 'number'
      )
        throw new Error('Missing compiler counter')
      return module.count
    },
    css(
      generation: Awaited<
        ReturnType<ReturnType<typeof createMdxSourceManager>['prepare']>
      >,
    ) {
      const engine = createWasm(root)
      generation.configureWasm(engine)
      for (const input of generation.inputs)
        extractInput(engine, settings, input)
      return engine.getCss(null, false)
    },
  }
}

export const styledMdx = `import { Box } from '@devup-ui/react'\n\n# Test\n\n<Box bg="red" />\n`
