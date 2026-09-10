import { existsSync, writeFileSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, join, relative, resolve } from 'node:path'

import {
  createThemeInterfaceArgs,
  type CustomShorthands,
  loadDevupConfig,
  mergeImportAliases,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  getCss,
  getThemeInterface,
  hasDevupUI,
  registerShorthands,
  registerTheme,
  setDebug,
} from '@devup-ui/wasm'
import { plugin } from 'bun'

import { cssDirName, cssNamespace, resolveCssId } from './css-id'

const libPackage = '@devup-ui/react'
const devupFile = 'devup.json'
const distDir = 'df'
const cssDir = resolve(distDir, cssDirName)
const singleCss = true
const importAliases = mergeImportAliases()

export interface DevupUIBunPluginOptions {
  shorthands?: CustomShorthands
}

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
  if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
  await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
  await writeDataFiles()
}

// Devup UI is a preprocessor: the stylesheet is a build artifact consumed by a
// bundler, and Bun's runtime has no CSS loader (`onLoad` only accepts the
// script/data loaders). The injected import exists so bundlers pick the
// stylesheet up, so under the Bun runtime it resolves to an empty module.
function loadCssModule() {
  return { contents: '', loader: 'js' as const }
}

async function loadSourceFile(filePath: string) {
  const loader: 'tsx' | 'ts' | 'jsx' | 'js' = filePath.endsWith('.tsx')
    ? 'tsx'
    : filePath.endsWith('.ts')
      ? 'ts'
      : filePath.endsWith('.jsx')
        ? 'jsx'
        : 'js'
  const contents = await Bun.file(filePath).text()

  if (hasDevupUI(filePath, contents, libPackage)) {
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
    // singleCss stores every extracted style in the base sheet. Finish the
    // write before returning the injected import; synchronous writes also keep
    // concurrent source loads from overwriting a newer sheet with an older one.
    writeFileSync(join(cssDir, 'devup-ui.css'), getCss(null, false), 'utf-8')
    return { contents: code.code, loader }
  }
  return { contents, loader }
}

// Registers the Bun plugin. Returns the promise produced by `plugin()` (its
// `setup` is async), so callers MUST `await` it. Bun's preload mechanism waits
// for an awaited module evaluation to settle; awaiting this guarantees the
// `onLoad` hook is installed before any source file is loaded. Without the
// await, preload-driven `bun test` users race the async setup and load sources
// against the @devup-ui/react runtime stubs (throwing "Cannot run on the
// runtime").
function register(options: DevupUIBunPluginOptions = {}) {
  return plugin({
    name: 'devup-ui',

    async setup(build) {
      await initialize(options)
      setDebug(true)

      // Resolve devup-ui CSS files onto a path-free virtual id, so nothing
      // derived from this checkout's cwd can be baked into Bun's shared,
      // content-keyed transpiler cache. See ./css-id.
      build.onResolve(
        { filter: /devup-ui(-\d+)?\.css$/ },
        ({ path, importer }) => resolveCssId(path, importer, distDir),
      )

      // Serve the virtual stylesheet resolved above
      build.onLoad({ filter: /.*/, namespace: cssNamespace }, () =>
        loadCssModule(),
      )

      // Load source files from packages directory (file namespace)
      build.onLoad(
        {
          filter: /\.(?:tsx?|jsx|mjs)$|[\\/]@devup-ui[\\/].*\.js$/,
        },
        ({ path }) => loadSourceFile(path),
      )
    },
  })
}

export { plugin, register }
