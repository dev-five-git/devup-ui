import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, join, resolve } from 'node:path'

import {
  beginBuild,
  collectNumberedFiles,
  createCompatTypes,
  createModuleResolver,
  createThemeInterfaceArgs,
  type CustomShorthands,
  isMdxSource,
  listSourceFiles,
  loadDevupConfig,
  mdxSourceFilter,
  normalizeMdxExtensions,
  type PreparedSource,
  resolveProjectPaths,
  resolveSourceDirs,
  seedFileNumbers,
  selectedSourceFilter,
} from '@devup-ui/plugin-utils'
import {
  getCss,
  getThemeInterface,
  registerShorthands,
  registerTheme,
  resetBuildState,
  seedFileMap,
  setDebug,
  setModuleResolver,
  setPrefix,
} from '@devup-ui/wasm'
import { type BunPlugin, type PluginBuilder } from 'bun'

import { cssNamespace, resolveCssId } from './css-id'
import { createMdxOwnership } from './mdx-ownership'
import {
  compiledPackages,
  importAliases,
  libPackage,
  mentionsCompiledPackage,
  runtimeSourceFilter,
} from './source'
import { loadSourceFile, type SourceProject } from './source-load'

type PluginHooks = {
  readonly config?: PluginBuilder['config']
  readonly onEnd?: (...args: Parameters<PluginBuilder['onEnd']>) => void
  readonly onLoad: (...args: Parameters<PluginBuilder['onLoad']>) => void
  readonly onResolve: (...args: Parameters<PluginBuilder['onResolve']>) => void
}

export interface DevupUIBunPluginOptions {
  shorthands?: CustomShorthands
  /** Resolved against Bun.build's root, or cwd in the runtime. */
  root?: string
  devupFile?: string
  distDir?: string
  sourceDirs?: string | string[]
  /** Uncompiled dependencies to transform under the runtime. */
  include?: string[]
  /** Readable class names. Defaults to false in both runtime and builds. */
  debug?: boolean
  mdxExtensions?: readonly string[]
}

type Project = ReturnType<typeof resolveProjectPaths> & SourceProject

async function writeDataFiles({ devupFile, distDir, cssDir }: Project) {
  const config = await loadDevupConfig(devupFile)
  registerTheme(config.theme ?? {})

  // Generate theme interface after registration (always write, even if empty)
  await writeFile(
    join(distDir, 'theme.d.ts'),
    getThemeInterface(...createThemeInterfaceArgs(libPackage)),
    'utf-8',
  )

  if (!existsSync(cssDir)) {
    await mkdir(cssDir, { recursive: true })
  }
  await writeFile(join(cssDir, 'devup-ui.css'), getCss(null, false), 'utf-8')
}

async function initialize(project: Project, options: DevupUIBunPluginOptions) {
  const { root, distDir } = project
  registerShorthands(options.shorthands ?? {})
  setPrefix(null)
  setModuleResolver(project.resolver)
  // Number every source file in path order, so class prefixes do not depend
  // on the order Bun loads files in
  try {
    seedFileNumbers(
      { seedFileMap },
      collectNumberedFiles({
        roots: resolveSourceDirs(root, options.sourceDirs),
        includeMdx: project.mdxExtensions,
        cwd: root,
        include: options.include,
        needles: compiledPackages,
        toId: (path) => path,
      }),
    )
  } catch (cause) {
    if (!(cause instanceof Error)) throw cause
    console.warn('[devup-ui] File numbering fallback', {
      root,
      phase: 'seed',
      impact: 'arrival-order file IDs',
      cause,
    })
  }
  if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
  await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
  await writeFile(
    join(distDir, 'compat.d.ts'),
    createCompatTypes(importAliases),
    'utf-8',
  )
  await writeDataFiles(project)
}

/**
 * The Devup UI plugin, for `Bun.build` (`plugins: [DevupUI()]`) as well as the
 * Bun runtime ({@link register}).
 */
export function DevupUI(options: DevupUIBunPluginOptions = {}) {
  return {
    name: 'devup-ui',

    async setup(build: PluginHooks) {
      // `Bun.build` hands its config to plugins; the runtime has none
      const bundling = build.config !== undefined
      const root = resolve(options.root ?? build.config?.root ?? process.cwd())
      const targetConditions = {
        browser: ['browser'],
        bun: ['bun', 'node'],
        node: ['node'],
      } as const
      const customConditions =
        typeof build.config?.conditions === 'string'
          ? [build.config.conditions]
          : (build.config?.conditions ?? [])
      const mdxExtensions = normalizeMdxExtensions(options.mdxExtensions)
      const conditions = [
        ...targetConditions[
          build.config?.target ?? (bundling ? 'browser' : 'bun')
        ],
        'import',
        ...(bundling ? ['module'] : []),
        ...customConditions,
      ]
      const prepared = new Map<string, PreparedSource>()
      const ownership = createMdxOwnership({
        root,
        extensions: mdxExtensions,
        conditions,
        entries: build.config?.entrypoints ?? [],
      })
      const project: Project = {
        ...resolveProjectPaths(root, options),
        root,
        debug: options.debug ?? false,
        mdxExtensions,
        prepared,
        ownership,
        resolver: createModuleResolver({
          cwd: root,
          // Extracted ESM imports stay import requests even in the vanilla
          // CommonJS evaluator. The callback does not expose a request kind.
          conditions,
          includeMdx: mdxExtensions,
          prepareSource: (filename) => prepared.get(filename),
        }),
      }
      // A build starts from its own options, not from what an earlier build in
      // this process left in the engine
      const endBuild = beginBuild(
        { resetBuildState },
        {
          integration: 'Bun',
          root,
        },
      )
      build.onEnd?.((result) => {
        try {
          if (result.success) ownership.validate()
        } finally {
          endBuild()
        }
      })
      await initialize(project, options)
      setDebug(project.debug)
      let writtenCss = getCss(null, false)
      // Native token modules must keep Bun's filesystem watch/CJS loading path.
      const sourceFilter = bundling
        ? selectedSourceFilter(mdxExtensions)
        : new RegExp(
            `${
              runtimeSourceFilter([
                ...listSourceFiles(
                  root,
                  [
                    'target',
                    'dist',
                    basename(project.distDir),
                    '.git',
                    'coverage',
                  ],
                  { includeMdx: mdxExtensions },
                ).filter(
                  (file) =>
                    isMdxSource(file, mdxExtensions) ||
                    mentionsCompiledPackage(readFileSync(file, 'utf-8')),
                ),
                ...collectNumberedFiles({
                  roots: resolveSourceDirs(root, options.sourceDirs),
                  includeMdx: mdxExtensions,
                  cwd: root,
                  include: ['@devup-ui/components', ...(options.include ?? [])],
                  needles: compiledPackages,
                  toId: (path) => path,
                }),
              ]).source
            }|${mdxSourceFilter(mdxExtensions).source}|\\.(?:test|spec)\\.[mc]?[jt]sx?$`,
            'i',
          )

      // Resolve devup-ui CSS files onto a path-free virtual id, so nothing
      // derived from this checkout's cwd can be baked into Bun's shared,
      // content-keyed transpiler cache. See ./css-id.
      build.onResolve(
        { filter: /devup-ui(-\d+)?\.css$/ },
        ({ path, importer }) =>
          resolveCssId(path, importer, basename(project.distDir)),
      )

      // The bundler takes the stylesheet once every other module is loaded,
      // so it holds the styles of all of them. The Bun runtime has no CSS
      // loader (`onLoad` only accepts the script/data loaders), so there the
      // injected import resolves to an empty module.
      build.onLoad(
        { filter: /.*/, namespace: cssNamespace },
        async ({ defer }) => {
          if (!bundling) return { contents: '', loader: 'js' }
          await defer()
          ownership.validate()
          return { contents: getCss(null, false), loader: 'css' }
        },
      )

      // Load source files from packages directory (file namespace)
      build.onLoad(
        {
          filter: sourceFilter,
        },
        ({ path }) => {
          return loadSourceFile(path, project).then((result) => {
            if (isMdxSource(path, mdxExtensions)) ownership.loaded(path)
            if (!bundling && result) {
              const css = getCss(null, false)
              // Read-after-import is synchronous; only identical revisions coalesce.
              if (css !== writtenCss) {
                writeFileSync(
                  join(project.cssDir, 'devup-ui.css'),
                  css,
                  'utf-8',
                )
                writtenCss = css
              }
            }
            return result
          })
        },
      )
    },
  } satisfies BunPlugin
}
