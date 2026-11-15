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
  isProjectJsx,
  loadCompiler,
  loadDevupConfig,
  mergeImportAliases,
  normalizeShorthands,
  readJsxImportSource,
  resolveDevupFile,
  resolveProjectPath,
  seedFileNumbers,
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
}

type SourceLoader = 'tsx' | 'ts' | 'jsx' | 'js'

async function writeDataFiles(
  compiler: DevupCompiler,
  { devupFile, distDir, cssDir }: BunProject,
): Promise<string[]> {
  let theme = {}
  let tailwindFiles: string[] = []
  try {
    const config = await loadDevupConfig(devupFile)
    theme = withTailwindCss(config.theme ?? {}, config)
    tailwindFiles = tailwindCssFiles(config)
  } catch {
    // Error reading devup.json, use empty theme
  }
  compiler.registerTheme(theme)

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
) {
  const { distDir } = project
  compiler.registerShorthands(shorthands)
  compiler.setPrefix(null)
  compiler.setModuleResolver(createModuleResolver())
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
  const loader: SourceLoader = filePath.endsWith('.tsx')
    ? 'tsx'
    : filePath.endsWith('.ts')
      ? 'ts'
      : filePath.endsWith('.jsx')
        ? 'jsx'
        : 'js'
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
    // every extracted style in the base sheet; synchronous writes keep
    // concurrent source loads from overwriting a newer sheet with an older one.
    if (!bundling)
      writeFileSync(
        join(cssDir, 'devup-ui.css'),
        compiler.getCss(null, false),
        'utf-8',
      )
    return { contents: code.code, loader }
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
  let project = locateProject(process.cwd())
  const watchTailwindCss = createTailwindWatcher(() =>
    writeDataFiles(compiler, project),
  )
  return {
    name: 'devup-ui',

    async setup(build: PluginBuilder) {
      // `Bun.build` hands its config to plugins; the runtime has none
      const bundling = build.config !== undefined
      // The relative paths belong to the `root` of `Bun.build` when it sets
      // one, not to the directory a monorepo tool starts the build from
      project = locateProject(
        resolve((bundling && build.config.root) || process.cwd()),
      )
      const { root, importAliases, cssDir } = project
      // A build starts from its own options, not from what an earlier build in
      // this process left in the engine
      const endBuild = beginBuild(compiler)
      build.onEnd?.(endBuild)
      await initialize(compiler, shorthands, watchTailwindCss, project)
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

      // Load source files from packages directory (file namespace)
      build.onLoad(
        {
          filter: /\.(?:tsx?|jsx|mjs)$|[\\/]@devup-ui[\\/].*\.js$/,
        },
        ({ path }) => {
          if (reservationPending) {
            reserveFiles()
            reservationPending = false
          }
          return loadSourceFile(compiler, path, bundling, project)
        },
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
