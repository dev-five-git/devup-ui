import { existsSync } from 'node:fs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { basename, join, resolve } from 'node:path'

import {
  codeExtract,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  registerTheme,
  setDebug,
} from '@devup-ui/wasm'
import type { RsbuildPlugin } from '@rsbuild/core'

export interface DevupUIRsbuildPluginOptions {
  package: string
  cssDir: string
  devupFile: string
  distDir: string
  extractCss: boolean
  debug: boolean
  include: string[]
  singleCss: boolean
}

let globalCss = ''

async function writeDataFiles(
  options: Omit<
    DevupUIRsbuildPluginOptions,
    'extractCss' | 'debug' | 'include'
  >,
) {
  try {
    const content = existsSync(options.devupFile)
      ? await readFile(options.devupFile, 'utf-8')
      : undefined

    if (content) {
      registerTheme(JSON.parse(content)?.['theme'] ?? {})
      const interfaceCode = getThemeInterface(
        options.package,
        'DevupThemeColors',
        'DevupThemeTypography',
        'DevupTheme',
      )

      if (interfaceCode) {
        await writeFile(
          join(options.distDir, 'theme.d.ts'),
          interfaceCode,
          'utf-8',
        )
      }
    } else {
      registerTheme({})
    }
  } catch (error) {
    console.error(error)
    registerTheme({})
  }
  await Promise.all([
    !existsSync(options.cssDir)
      ? mkdir(options.cssDir, { recursive: true })
      : Promise.resolve(),
    !options.singleCss
      ? writeFile(join(options.cssDir, 'devup-ui.css'), getCss(null, false))
      : Promise.resolve(),
  ])
}

export const DevupUI = ({
  include = [],
  package: libPackage = '@devup-ui/react',
  extractCss = true,
  distDir = 'df',
  cssDir = resolve(distDir, 'devup-ui'),
  devupFile = 'devup.json',
  debug = false,
  singleCss = false,
}: Partial<DevupUIRsbuildPluginOptions> = {}): RsbuildPlugin => ({
  name: 'devup-ui-rsbuild-plugin',
  async setup(api) {
    setDebug(debug)

    if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
    await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')

    await writeDataFiles({
      package: libPackage,
      cssDir,
      devupFile,
      distDir,
      singleCss,
    })
    if (!extractCss) return

    api.transform(
      {
        test: cssDir,
      },
      () => globalCss,
    )

    api.modifyRsbuildConfig((config) => {
      const theme = getDefaultTheme()
      if (theme) {
        config.source ??= {}
        config.source.define = {
          'process.env.DEVUP_UI_DEFAULT_THEME':
            JSON.stringify(getDefaultTheme()),
          ...config.source.define,
        }
      }
      return config
    })

    api.transform(
      {
        test: /\.(tsx|ts|js|mjs|jsx)$/,
      },
      async ({ code, resourcePath }) => {
        if (
          new RegExp(
            `node_modules(?!.*(${['@devup-ui', ...include]
              .join('|')
              .replaceAll('/', '[\\/\\\\_]')})([\\/\\\\.]|$))`,
          ).test(resourcePath)
        )
          return code
        const {
          code: retCode,
          css,
          map,
          cssFile,
          updatedBaseStyle,
        } = codeExtract(
          resourcePath,
          code,
          libPackage,
          cssDir,
          singleCss,
          false,
          true,
        )
        const promises: Promise<void>[] = []
        if (updatedBaseStyle) {
          // update base style
          promises.push(
            writeFile(
              join(cssDir, 'devup-ui.css'),
              getCss(null, false),
              'utf-8',
            ),
          )
        }

        if (css) {
          if (globalCss.length < css.length) globalCss = css
          promises.push(
            writeFile(
              join(cssDir, basename(cssFile!)),
              `/* ${resourcePath} ${Date.now()} */`,
              'utf-8',
            ),
          )
        }
        await Promise.all(promises)
        return {
          code: retCode,
          map,
        }
      },
    )
  },
})
