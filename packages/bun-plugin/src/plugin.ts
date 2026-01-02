import { existsSync } from 'node:fs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import * as wasmModule from '@devup-ui/wasm'
import { plugin } from 'bun'

const libPackage = '@devup-ui/react'
const devupFile = 'devup.json'
const distDir = 'df'
const cssDir = resolve(distDir, 'devup-ui')
const singleCss = true

let wasmInitialized = false

function getWasm() {
  if (!wasmInitialized) {
    wasmModule.setDebug(true)
    wasmInitialized = true
  }
  return wasmModule
}

async function writeDataFiles() {
  const wasm = await getWasm()
  try {
    const content = existsSync(devupFile)
      ? await readFile(devupFile, 'utf-8')
      : undefined

    if (content) {
      wasm.registerTheme(JSON.parse(content)?.['theme'] ?? {})
      const interfaceCode = wasm.getThemeInterface(
        libPackage,
        'CustomColors',
        'DevupThemeTypography',
        'DevupTheme',
      )

      if (interfaceCode) {
        await writeFile(join(distDir, 'theme.d.ts'), interfaceCode, 'utf-8')
      }
    } else {
      wasm.registerTheme({})
    }
  } catch (error) {
    console.error(error)
    wasm.registerTheme({})
  }
  await Promise.all([
    !existsSync(cssDir)
      ? mkdir(cssDir, { recursive: true })
      : Promise.resolve(),
    !singleCss
      ? writeFile(join(cssDir, 'devup-ui.css'), wasm.getCss(null, false))
      : Promise.resolve(),
  ])
}

let initialized = false

async function initialize() {
  if (initialized) return
  initialized = true

  if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
  await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
  await writeDataFiles()
}

// Register plugin immediately before any other imports
plugin({
  name: 'devup-ui',

  async setup(build) {
    await initialize()

    // Resolve devup-ui CSS files
    build.onResolve(
      { filter: /devup-ui(-\d+)?\.css$/ },
      ({ path, importer }) => {
        const fileName = basename(path).split('?')[0]
        const resolvedPath = importer
          ? resolve(dirname(importer), path)
          : resolve(path)
        const expectedPath = resolve(join(cssDir, fileName))
        // console.log('wtf', resolvedPath, expectedPath)

        if (!relative(resolvedPath, expectedPath) || path.startsWith(cssDir)) {
          // Return external to skip CSS in test environment
          return {
            path: join(cssDir, fileName),
          }
        }
      },
    )

    // Load CSS files
    // build.onLoad({ filter: /\.css/ }, async ({ path }) => {
    //   console.log('wtf22', path)
    //   const fileName = basename(path).split('?')[0]
    //   if (!/devup-ui(-\d+)?\.css$/.test(fileName)) {
    //     return { contents: '', loader: 'css' }
    //   }

    //   const fileNum = getFileNumByFilename(fileName)
    //   const css = getCss(fileNum, false)
    //   cssMap.set(fileNum, css)
    //   return {
    //     contents: css,
    //     loader: 'css',
    //   }
    // })

    // Load source files from packages directory (file namespace)
    build.onLoad({ filter: /.*\.(tsx|ts|jsx|mjs)/ }, async ({ path }) => {
      const filePath = path
      const loader = filePath.endsWith('.tsx')
        ? 'tsx'
        : filePath.endsWith('.ts')
          ? 'ts'
          : filePath.endsWith('.jsx')
            ? 'jsx'
            : 'js'
      const contents = await Bun.file(filePath).text()

      const wasm = await getWasm()
      if (wasm.hasDevupUI(filePath, contents, libPackage)) {
        const code = wasm.codeExtract(
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
    })
  },
})
