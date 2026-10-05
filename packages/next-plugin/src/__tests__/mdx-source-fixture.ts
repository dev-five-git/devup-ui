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

const workspace = resolve(import.meta.dir, '../../../..')
const installedRoot = join(workspace, 'apps/landing')
const installed = createRequire(join(installedRoot, 'package.json'))
const roots: string[] = []
afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
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
  const plugin = write(
    'reporting-remark.cjs',
    `
const fs = require('node:fs'); const path = require('node:path');
let context; const counts = new Map();
function remark(options) { return async function(tree, file) {
  counts.set(file.path, (counts.get(file.path) || 0) + 1);
  if (options.before) await options.before(file.path);
  const data = options.data || path.join(path.dirname(file.path), 'data.json');
  if (fs.existsSync(data) && !tree.children.some(child => child.type === 'mdxjsEsm' && child.value.includes('css'))) tree.children.unshift({type:'mdxjsEsm',value:'',data:{estree:{type:'Program',sourceType:'module',body:[{type:'ImportDeclaration',source:{type:'Literal',value:'@devup-ui/react'},specifiers:[{type:'ImportSpecifier',local:{type:'Identifier',name:'css'},imported:{type:'Identifier',name:'css'}}]}]}}});
  if (fs.existsSync(data)) { context.addDependency(data); const value = JSON.parse(fs.readFileSync(data, 'utf8')); tree.children.push({type:'mdxjsEsm',value:'',data:{estree:{type:'Program',sourceType:'module',body:[{type:'ExportNamedDeclaration',specifiers:[],declaration:{type:'VariableDeclaration',kind:'const',declarations:[{type:'VariableDeclarator',id:{type:'Identifier',name:'injectedStyle'},init:{type:'CallExpression',callee:{type:'Identifier',name:'css'},arguments:[{type:'ObjectExpression',properties:[{type:'Property',kind:'init',computed:false,method:false,shorthand:false,key:{type:'Identifier',name:'color'},value:{type:'Literal',value:value.color}}]}]}}]}}]}}}); }
  const directory = path.join(path.dirname(file.path), 'reported');
  if (fs.existsSync(directory)) { context.addContextDependency(directory); tree.children.push({type:'paragraph',children:[{type:'text',value:fs.readdirSync(directory).join(',')}]}); }
  const missing = path.join(path.dirname(file.path), 'optional.json');
  if (!fs.existsSync(missing)) context.addMissingDependency(missing); else context.addDependency(missing);
  const build = path.join(path.dirname(file.path), 'build.json'); if (fs.existsSync(build)) context.addBuildDependency(build);
} }
remark.setContext = value => { context = value }; remark.counts = counts; module.exports = remark;
`,
  )
  const raw = write(
    'reporting-raw.cjs',
    `const seen = []; module.exports = function(source) { seen.push(this.loaders[0].options); require(this.getOptions().plugin).setContext(this); return source }; module.exports.seen = seen`,
  )
  const pluginOptions: {
    before?: (filename: string) => Promise<void>
    data?: string
  } = {}
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
    counts: () => {
      const module: unknown = createRequire(join(root, 'package.json'))(plugin)
      if (
        typeof module !== 'function' ||
        !('counts' in module) ||
        !(module.counts instanceof Map)
      )
        throw new TypeError('Missing compiler counter')
      return [...module.counts.values()].reduce(
        (sum: number, value: number) => sum + value,
        0,
      )
    },
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
