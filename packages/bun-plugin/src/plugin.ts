import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

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

const libPackage = '@devup-ui/react'
const devupFile = 'devup.json'
const distDir = 'df'
const cssDir = resolve(distDir, 'devup-ui')
const baseCssFile = join(cssDir, 'devup-ui.css')
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
  // `onResolve` points every devup-ui.css import at this file, so it has to
  // exist before the first source file is loaded. Without it the very first
  // import fails to resolve and the whole run dies before any style is
  // collected.
  if (!existsSync(baseCssFile)) {
    await writeFile(baseCssFile, getCss(null, false), 'utf-8')
  }
}

async function writeBaseCss() {
  await writeFile(baseCssFile, getCss(null, false), 'utf-8')
}

async function initialize({ shorthands }: DevupUIBunPluginOptions = {}) {
  registerShorthands(shorthands ?? {})
  if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
  await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
  await writeDataFiles()
}

function resolveCssPath(path: string, importer?: string) {
  const fileName = basename(path).split('?')[0]
  const resolvedPath = importer
    ? resolve(dirname(importer), path)
    : resolve(path)
  const expectedPath = resolve(join(cssDir, fileName))

  if (!relative(resolvedPath, expectedPath) || path.startsWith(cssDir)) {
    return { path: join(cssDir, fileName) }
  }
  return undefined
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
    const { code, updatedBaseStyle } = codeExtract(
      filePath,
      contents,
      libPackage,
      relative(dirname(filePath), cssDir).replaceAll('\\', '/'),
      singleCss,
      true,
      false,
      importAliases,
    )
    // The extracted styles only reach the browser if they are written out.
    // Every other plugin does this; here they were collected and dropped, so
    // the file the imports resolve to stayed whatever it was.
    if (updatedBaseStyle) await writeBaseCss()
    return { contents: code, loader }
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

      // Resolve devup-ui CSS files
      build.onResolve(
        { filter: /devup-ui(-\d+)?\.css$/ },
        ({ path, importer }) => resolveCssPath(path, importer),
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
