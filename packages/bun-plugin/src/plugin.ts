import { existsSync } from 'node:fs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  codeExtract,
  getCss,
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
  try {
    const content = existsSync(devupFile)
      ? await readFile(devupFile, 'utf-8')
      : undefined

    if (content) {
      registerTheme(JSON.parse(content)?.['theme'] ?? {})
      const interfaceCode = getThemeInterface(
        libPackage,
        'CustomColors',
        'DevupThemeTypography',
        'DevupTheme',
      )

      if (interfaceCode) {
        await writeFile(join(distDir, 'theme.d.ts'), interfaceCode, 'utf-8')
      }
    } else {
      registerTheme({})
    }
  } catch (error) {
    console.error(error)
    registerTheme({})
  }
  await Promise.all([
    !existsSync(cssDir)
      ? mkdir(cssDir, { recursive: true })
      : Promise.resolve(),
    !singleCss
      ? writeFile(join(cssDir, 'devup-ui.css'), getCss(null, false))
      : Promise.resolve(),
  ])
}

async function initialize() {
  if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
  await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
  await writeDataFiles()
}

// Register plugin immediately before any other imports
plugin({
  name: 'devup-ui',

  async setup(build) {
    await initialize()
    setDebug(true)

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
    })
  },
})
