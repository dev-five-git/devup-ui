import { existsSync, writeFileSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, join, relative, resolve } from 'node:path'

import {
  createCompatTypes,
  createModuleResolver,
  createThemeInterfaceArgs,
  type CustomShorthands,
  loadDevupConfig,
  mergeImportAliases,
  readJsxImportSource,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  getCss,
  getThemeInterface,
  hasDevupUI,
  registerShorthands,
  registerTheme,
  setDebug,
  setModuleResolver,
} from '@devup-ui/wasm'
import { type BunPlugin, plugin, type PluginBuilder } from 'bun'

import { cssDirName, cssNamespace, resolveCssId } from './css-id'

const libPackage = '@devup-ui/react'
const devupFile = 'devup.json'
const distDir = 'df'
const cssDir = resolve(distDir, cssDirName)
const singleCss = true
const importAliases = mergeImportAliases(undefined, readJsxImportSource())
// The packages whose imports the extractor compiles: Devup UI, the packages it
// takes the place of, and StyleX
const compiledPackages = [
  libPackage,
  '@stylexjs/stylex',
  ...Object.keys(importAliases),
]

export interface DevupUIBunPluginOptions {
  shorthands?: CustomShorthands
  /**
   * Readable class names. Defaults to `true` under the Bun runtime (tests) and
   * `false` in `Bun.build`.
   */
  debug?: boolean
}

type SourceLoader = 'tsx' | 'ts' | 'jsx' | 'js'

async function writeDataFiles() {
  let theme = {}
  try {
    const config = await loadDevupConfig(devupFile)
    theme = config.theme ?? {}
  } catch {
    // Error reading devup.json, use empty theme
  }
  registerTheme(theme)

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

async function initialize({ shorthands }: DevupUIBunPluginOptions = {}) {
  registerShorthands(shorthands ?? {})
  setModuleResolver(createModuleResolver())
  if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
  await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
  await writeFile(
    join(distDir, 'compat.d.ts'),
    createCompatTypes(importAliases),
    'utf-8',
  )
  await writeDataFiles()
}

const scanners = new Map<SourceLoader, Bun.Transpiler>()

/** Whether `contents` imports a package the extractor compiles */
function importsCompiledPackage(contents: string, loader: SourceLoader) {
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

async function loadSourceFile(filePath: string, bundling: boolean) {
  const loader: SourceLoader = filePath.endsWith('.tsx')
    ? 'tsx'
    : filePath.endsWith('.ts')
      ? 'ts'
      : filePath.endsWith('.jsx')
        ? 'jsx'
        : 'js'
  const contents = await Bun.file(filePath).text()

  if (
    importsCompiledPackage(contents, loader) ||
    hasDevupUI(filePath, contents, libPackage, importAliases)
  ) {
    const code = codeExtract(
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
      writeFileSync(join(cssDir, 'devup-ui.css'), getCss(null, false), 'utf-8')
    return { contents: code.code, loader }
  }
  return { contents, loader }
}

/**
 * The Devup UI plugin, for `Bun.build` (`plugins: [DevupUI()]`) as well as the
 * Bun runtime ({@link register}).
 */
function DevupUI(options: DevupUIBunPluginOptions = {}) {
  return {
    name: 'devup-ui',

    async setup(build: PluginBuilder) {
      // `Bun.build` hands its config to plugins; the runtime has none
      const bundling = build.config !== undefined
      await initialize(options)
      setDebug(options.debug ?? !bundling)

      // Resolve devup-ui CSS files onto a path-free virtual id, so nothing
      // derived from this checkout's cwd can be baked into Bun's shared,
      // content-keyed transpiler cache. See ./css-id.
      build.onResolve(
        { filter: /devup-ui(-\d+)?\.css$/ },
        ({ path, importer }) => resolveCssId(path, importer, distDir),
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
          filter: /\.(?:tsx?|jsx|mjs)$|[\\/]@devup-ui[\\/].*\.js$/,
        },
        ({ path }) => loadSourceFile(path, bundling),
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
