import { existsSync, writeFileSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, join, relative, resolve } from 'node:path'

import {
  beginBuild,
  collectNumberedFiles,
  createCompatTypes,
  createModuleResolver,
  createSharedSlotReservation,
  createThemeInterfaceArgs,
  type CustomShorthands,
  type DevupCompiler,
  type DevupEngine,
  isMissingDirectory,
  isProjectJsx,
  loadCompiler,
  loadDevupConfig,
  mergeImportAliases,
  normalizeShorthands,
  readJsxImportSource,
  resolveDevupFile,
  resolveProjectPath,
  seedFileNumbers,
  setupWarning,
  tailwindCssFiles,
  type WasmImportAliases,
  withTailwindCss,
} from '@devup-ui/plugin-utils'
import { type BunPlugin, plugin, type PluginBuilder } from 'bun'

import { cssDirName, cssNamespace, resolveCssId } from './css-id'
import { createTailwindWatcher } from './tailwind-watch'

const libPackage = '@devup-ui/react'
const distDirName = 'df'
const singleCss = true

/** Where a project keeps its files, and what its tsconfig compiles */
interface BunProject {
  readonly root: string
  readonly devupFile: string
  readonly distDir: string
  readonly cssDir: string
  readonly importAliases: WasmImportAliases
  /**
   * The packages whose imports the extractor compiles: Devup UI, the packages
   * it takes the place of, and StyleX
   */
  readonly compiledPackages: string[]
}

/**
 * The project at `root`: the `root` of `Bun.build`, or the working directory
 * of the runtime and of a build without one
 */
function locateProject(root: string): BunProject {
  const importAliases = mergeImportAliases(undefined, readJsxImportSource(root))
  return {
    root,
    devupFile: resolveDevupFile(root, undefined),
    distDir: resolveProjectPath(root, distDirName),
    cssDir: resolve(root, distDirName, cssDirName),
    importAliases,
    compiledPackages: [
      libPackage,
      '@stylexjs/stylex',
      ...Object.keys(importAliases),
    ],
  }
}

export interface DevupUIBunPluginOptions {
  /** Native by default. Select WASM explicitly without automatic fallback. */
  engine?: DevupEngine
  shorthands?: CustomShorthands
  /**
   * Readable class names. Defaults to `true` under the Bun runtime (tests) and
   * `false` in `Bun.build`.
   */
  debug?: boolean
  /**
   * Packages whose published `.js` files are compiled as well, as in the
   * other plugins: a library that ships Devup UI components uncompiled.
   * Each name matches that whole package (`pkg` is not `pkg-extra` or
   * `@scope/pkg`), not the dependencies in its own `node_modules`.
   */
  include?: readonly string[]
}

type SourceLoader = 'tsx' | 'ts' | 'jsx' | 'js'

const SEPARATOR = '[\\/\\\\]'

/**
 * The files the plugin loads: TypeScript (`.mts` and `.cts` included) and JSX
 * sources anywhere, `.js` under Devup UI, and `.js` of the packages in
 * `include`. An included package is the directory right under the innermost
 * `node_modules` (pnpm and Bun stores included), matched the way
 * `createNodeModulesExcludeRegex` matches it.
 */
function createSourceFilter(include: readonly string[]): RegExp {
  const sources = `\\.(?:[mc]?ts|tsx|jsx|mjs)$|${SEPARATOR}@devup-ui${SEPARATOR}.*\\.js$`
  if (include.length === 0) return new RegExp(sources)
  const names = include
    .map((name) =>
      name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replaceAll('/', SEPARATOR),
    )
    .join('|')
  return new RegExp(
    `${sources}|${SEPARATOR}node_modules${SEPARATOR}(?!.*${SEPARATOR}node_modules${SEPARATOR})(?:${names})${SEPARATOR}.*\\.js$`,
  )
}

/**
 * The `exports` conditions Bun resolves an import with: those of its target
 * (`browser` by default for `Bun.build`, `bun` under the runtime) and the
 * configured `conditions`
 */
export function bunConditions(
  config: Pick<Bun.BuildConfig, 'target' | 'conditions'> | undefined,
): string[] {
  const target = config ? (config.target ?? 'browser') : 'bun'
  return [
    'import',
    'module',
    ...{ browser: ['browser'], node: ['node'], bun: ['bun', 'node'] }[target],
    ...[config?.conditions ?? []].flat(),
  ]
}

/**
 * `code` followed by a bare import of each build-time dependency, so that
 * Bun's module graph, and with it its watcher, keeps the modules whose values
 * the extractor inlined. Appended, the imports move no line or column of the
 * extracted code.
 */
export function withDependencyImports(
  code: string,
  filePath: string,
  dependencies: readonly string[],
): string {
  return [
    code,
    ...[...new Set(dependencies)].map((dependency) => {
      const path = relative(dirname(filePath), dependency).replaceAll('\\', '/')
      return `import ${JSON.stringify(path.startsWith('.') ? path : `./${path}`)};`
    }),
  ].join('\n')
}

/** Bun's loader for a source file */
function sourceLoader(filePath: string): SourceLoader {
  if (filePath.endsWith('.tsx')) return 'tsx'
  if (/\.[mc]?ts$/.test(filePath)) return 'ts'
  if (filePath.endsWith('.jsx')) return 'jsx'
  return 'js'
}

/**
 * Load devup.json into the compiler. The first load fails on a theme that
 * cannot be loaded or registered: Bun runs once, and an empty theme would
 * leave every token unstyled. A reload for a changed Tailwind CSS runs in a
 * process that is already up, so it reports the error and clears the theme.
 */
async function writeDataFiles(
  compiler: DevupCompiler,
  { devupFile, distDir, cssDir }: BunProject,
  reloading = false,
): Promise<string[]> {
  let tailwindFiles: string[] = []
  try {
    const config = await loadDevupConfig(devupFile)
    tailwindFiles = tailwindCssFiles(config)
    compiler.registerTheme(withTailwindCss(config.theme ?? {}, config))
  } catch (error) {
    if (!reloading) throw error
    console.error(error)
    compiler.registerTheme({})
  }

  // Generate theme interface after registration (always write, even if empty)
  await writeFile(
    join(distDir, 'theme.d.ts'),
    compiler.getThemeInterface(...createThemeInterfaceArgs(libPackage)),
    'utf-8',
  )

  if (!existsSync(cssDir)) {
    await mkdir(cssDir, { recursive: true })
  }
  await writeFile(
    join(cssDir, 'devup-ui.css'),
    compiler.getCss(null, false),
    'utf-8',
  )
  return tailwindFiles
}

async function initialize(
  compiler: DevupCompiler,
  shorthands: Record<string, readonly string[]>,
  watchTailwindCss: ReturnType<typeof createTailwindWatcher>,
  project: BunProject,
  conditions: readonly string[],
) {
  const { distDir } = project
  compiler.registerShorthands(shorthands)
  compiler.setPrefix(null)
  compiler.setModuleResolver(createModuleResolver({ conditions }))
  if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
  await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
  await writeFile(
    join(distDir, 'compat.d.ts'),
    createCompatTypes(project.importAliases),
    'utf-8',
  )
  watchTailwindCss(await writeDataFiles(compiler, project))
}

const scanners = new Map<SourceLoader, Bun.Transpiler>()

/** Whether `contents` imports a package the extractor compiles */
function importsCompiledPackage(
  contents: string,
  loader: SourceLoader,
  compiledPackages: readonly string[],
) {
  let scanner = scanners.get(loader)
  if (!scanner) {
    scanner = new Bun.Transpiler({ loader })
    scanners.set(loader, scanner)
  }
  try {
    return scanner
      .scanImports(contents)
      .some(({ path }) =>
        compiledPackages.some(
          (name) => path === name || path.startsWith(`${name}/`),
        ),
      )
  } catch {
    // Bun reports the syntax error when it loads the untouched source
    return false
  }
}

async function loadSourceFile(
  compiler: DevupCompiler,
  filePath: string,
  bundling: boolean,
  { cssDir, importAliases, compiledPackages }: BunProject,
) {
  const loader = sourceLoader(filePath)
  const contents = await Bun.file(filePath).text()

  if (
    isProjectJsx(filePath, importAliases) ||
    importsCompiledPackage(contents, loader, compiledPackages) ||
    compiler.hasDevupUI(filePath, contents, libPackage, importAliases)
  ) {
    using code = compiler.codeExtract(
      filePath,
      contents,
      libPackage,
      relative(dirname(filePath), cssDir).replaceAll('\\', '/'),
      singleCss,
      true,
      false,
      importAliases,
    )
    // Under the runtime the stylesheet is read from disk. singleCss stores
    // every extracted style in the base sheet, which the output carries only
    // when this module changed it, so a module that adds no style writes
    // nothing. Synchronous writes keep concurrent source loads from
    // overwriting a newer sheet with an older one.
    const sheet = code.css
    if (!bundling && sheet !== undefined)
      writeFileSync(join(cssDir, 'devup-ui.css'), sheet, 'utf-8')
    return {
      contents: withDependencyImports(code.code, filePath, code.dependencies),
      loader,
    }
  }
  return { contents, loader }
}

/**
 * The Devup UI plugin, for `Bun.build` (`plugins: [DevupUI()]`) as well as the
 * Bun runtime ({@link register}).
 */
function DevupUI(options: DevupUIBunPluginOptions = {}) {
  const shorthands = normalizeShorthands(options.shorthands ?? {})
  const compiler = loadCompiler(options.engine)
  // Each setup locates the project again; the watcher reads it then
  const project = locateProject(process.cwd())
  const watchTailwindCss = createTailwindWatcher(() =>
    writeDataFiles(compiler, project, true),
  )
  return {
    name: 'devup-ui',

    async setup(build: PluginBuilder) {
      // `Bun.build` hands its config to plugins; the runtime has none
      const bundling = build.config !== undefined
      // The relative paths belong to the `root` of `Bun.build` when it sets
      // one, not to the directory a monorepo tool starts the build from
      Object.assign(
        project,
        locateProject(
          resolve((bundling && build.config.root) || process.cwd()),
        ),
      )
      const { root, importAliases, cssDir } = project
      // A build starts from its own options, not from what an earlier build in
      // this process left in the engine
      const endBuild = beginBuild(compiler)
      build.onEnd?.(endBuild)
      await initialize(
        compiler,
        shorthands,
        watchTailwindCss,
        project,
        bunConditions(build.config),
      )
      const debug = options.debug ?? !bundling
      compiler.setDebug(debug)
      const slots = createSharedSlotReservation(compiler)
      let reservationPending = bundling
      const reserveFiles = () => {
        slots.invalidate()
        slots.reserve(
          {
            roots: [resolve(root, 'src'), resolve(root, 'app')],
            entries: bundling
              ? build.config.entrypoints.map((entry) => ({
                  request: resolve(entry),
                  context: root,
                }))
              : [],
            cwd: root,
            package: libPackage,
            importAliases,
            toId: (path) => path,
          },
          {
            singleCss,
            importMainCssInCode: true,
            importMainCssInCss: false,
            cssDir: ({ physicalPath }) =>
              relative(dirname(physicalPath), cssDir).replaceAll('\\', '/'),
          },
        )
      }
      if (bundling)
        build.onStart(() => {
          reservationPending = true
        })
      else if (!debug) reserveFiles()
      else {
        try {
          seedFileNumbers(
            compiler,
            collectNumberedFiles({
              roots: [resolve(root, 'src')],
              cwd: root,
              needles: project.compiledPackages,
              importAliases,
              toId: (path) => path,
            }),
          )
        } catch (error) {
          if (!(error instanceof Error)) throw error
          if (!isMissingDirectory(error, resolve(root, 'src')))
            console.warn(
              setupWarning(
                'Bun file numbering',
                error,
                'Stylesheet file numbers follow the order Bun loads files in, so they can differ between runs of the same source.',
              ),
            )
        }
      }

      // Resolve devup-ui CSS files onto a path-free virtual id, so nothing
      // derived from this checkout's cwd can be baked into Bun's shared,
      // content-keyed transpiler cache. See ./css-id.
      build.onResolve(
        { filter: /devup-ui(-\d+)?\.css$/ },
        ({ path, importer }) => resolveCssId(path, importer, distDirName),
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
          return { contents: compiler.getCss(null, false), loader: 'css' }
        },
      )

      const loadSource = ({ path }: { readonly path: string }) => {
        if (reservationPending) {
          reserveFiles()
          reservationPending = false
        }
        return loadSourceFile(compiler, path, bundling, project)
      }
      // Load source files from packages directory (file namespace)
      build.onLoad(
        { filter: createSourceFilter(options.include ?? []) },
        loadSource,
      )
      // A bundle also takes the project's own `.js` and `.cjs` modules. Bun
      // loads one itself when the hook returns nothing, which the runtime
      // does not allow, so dependencies are left to Bun here.
      if (bundling)
        build.onLoad({ filter: /\.c?js$/ }, (args) =>
          !/[\\/]node_modules[\\/]/.test(args.path)
            ? loadSource(args)
            : undefined,
        )
    },
  } satisfies BunPlugin
}

// Registers the Bun runtime plugin. Returns the promise produced by `plugin()`
// (its `setup` is async), so callers MUST `await` it. Bun's preload mechanism
// waits for an awaited module evaluation to settle; awaiting this guarantees
// the `onLoad` hook is installed before any source file is loaded. Without the
// await, preload-driven `bun test` users race the async setup and load sources
// against the @devup-ui/react runtime stubs (throwing "Cannot run on the
// runtime").
function register(options: DevupUIBunPluginOptions = {}) {
  return plugin(DevupUI(options))
}

export { DevupUI, plugin, register }
