import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  beginBuild,
  collectNumberedFiles,
  createCompatTypes,
  createModuleResolver,
  createThemeInterfaceArgs,
  type CustomShorthands,
  GRAPH_SOURCE_FILE_RE,
  listSourceFiles,
  loadDevupConfig,
  MDX_FILE_RE,
  remapMdxError,
  resolveProjectPaths,
  resolveSourceDirs,
  seedFileNumbers,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
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
import { compileMdx } from './mdx'
import {
  compiledPackages,
  importAliases,
  importsCompiledPackage,
  libPackage,
  mentionsCompiledPackage,
  preserveDependencies,
  runtimeSourceFilter,
  sourceLoader,
} from './source'

const singleCss = true

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
}

type Project = ReturnType<typeof resolveProjectPaths> & {
  readonly root: string
  readonly debug: boolean
  readonly resolver: ReturnType<typeof createModuleResolver>
}

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
        includeMdx: true,
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

async function loadSourceFile(filePath: string, project: Project) {
  const original = await Bun.file(filePath).text()
  const mdx = MDX_FILE_RE.test(filePath)
    ? await compileMdx(project.root, filePath, original)
    : undefined
  if (MDX_FILE_RE.test(filePath) && !mdx) return undefined
  const loader = mdx ? 'jsx' : sourceLoader(filePath)
  const contents = mdx?.value ?? original

  if (importsCompiledPackage(contents, loader)) {
    setDebug(project.debug)
    setModuleResolver(project.resolver)
    try {
      const code = codeExtract(
        filePath,
        contents,
        libPackage,
        relative(dirname(filePath), project.cssDir).replaceAll('\\', '/'),
        singleCss,
        true,
        false,
        importAliases,
      )
      return {
        contents: preserveDependencies(code.code, filePath, code.dependencies),
        loader,
      }
    } catch (cause) {
      if (mdx) throw remapMdxError(cause, filePath, mdx.map)
      throw cause
    }
  }
  return { contents, loader }
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
      const project: Project = {
        ...resolveProjectPaths(root, options),
        root,
        debug: options.debug ?? false,
        resolver: createModuleResolver({
          cwd: root,
          // Extracted ESM imports stay import requests even in the vanilla
          // CommonJS evaluator. The callback does not expose a request kind.
          conditions: [
            ...targetConditions[
              build.config?.target ?? (bundling ? 'browser' : 'bun')
            ],
            'import',
            ...(bundling ? ['module'] : []),
            ...customConditions,
          ],
        }),
      }
      // A build starts from its own options, not from what an earlier build in
      // this process left in the engine
      const endBuild = beginBuild({ resetBuildState })
      build.onEnd?.(endBuild)
      await initialize(project, options)
      setDebug(project.debug)
      let writtenCss = getCss(null, false)
      // Native token modules must keep Bun's filesystem watch/CJS loading path.
      const sourceFilter = bundling
        ? GRAPH_SOURCE_FILE_RE
        : new RegExp(
            `${
              runtimeSourceFilter([
                ...listSourceFiles(root, [
                  'target',
                  'dist',
                  basename(project.distDir),
                  '.git',
                  'coverage',
                ]).filter((file) =>
                  mentionsCompiledPackage(readFileSync(file, 'utf-8')),
                ),
                ...collectNumberedFiles({
                  roots: resolveSourceDirs(root, options.sourceDirs),
                  includeMdx: true,
                  cwd: root,
                  include: ['@devup-ui/components', ...(options.include ?? [])],
                  needles: compiledPackages,
                  toId: (path) => path,
                }),
              ]).source
            }|\\.(?:test|spec)\\.[mc]?[jt]sx?$`,
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
