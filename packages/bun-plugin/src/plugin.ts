import { existsSync } from 'node:fs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  codeExtract,
  getThemeInterface,
  hasDevupUI,
  registerTheme,
  setDebug,
} from '@devup-ui/wasm'
import { plugin } from 'bun'

const libPackage = '@devup-ui/react'
const devupFile = 'devup.json'
const distDir = 'df'
const cssDir = resolve(distDir, 'devup-ui')
const singleCss = true

async function writeDataFiles() {
  let theme = {}
  try {
    const content = existsSync(devupFile)
      ? await readFile(devupFile, 'utf-8')
      : undefined

    if (content) {
      theme = JSON.parse(content)?.['theme'] ?? {}
    }
  } catch {
    // Error reading devup.json, use empty theme
  }
  registerTheme(theme)

  // Generate theme interface after registration (always write, even if empty)
  await writeFile(
    join(distDir, 'theme.d.ts'),
    getThemeInterface(
      libPackage,
      'CustomColors',
      'DevupThemeTypography',
      'DevupTheme',
    ),
    'utf-8',
  )

  if (!existsSync(cssDir)) {
    await mkdir(cssDir, { recursive: true })
  }
}

async function initialize() {
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
    const code = codeExtract(
      filePath,
      contents,
      libPackage,
      relative(dirname(filePath), cssDir).replaceAll('\\', '/'),
      singleCss,
      true,
      false,
    )
    return { contents: code.code, loader }
  }
  return { contents, loader }
}

// Register plugin immediately before any other imports
plugin({
  name: 'devup-ui',

  async setup(build) {
    await initialize()
    setDebug(true)

    // Resolve devup-ui CSS files
    build.onResolve({ filter: /devup-ui(-\d+)?\.css$/ }, ({ path, importer }) =>
      resolveCssPath(path, importer),
    )

    // Load source files from packages directory (file namespace)
    build.onLoad({ filter: /.*\.(tsx|ts|jsx|mjs)/ }, ({ path }) =>
      loadSourceFile(path),
    )
  },
})

export { plugin }
