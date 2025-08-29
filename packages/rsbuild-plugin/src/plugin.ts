import { existsSync } from 'node:fs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { basename, join, resolve } from 'node:path'

import {
  codeExtract,
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
    'extractCss' | 'debug' | 'include' | 'singleCss'
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
        await writeFile(join(options.distDir, 'theme.d.ts'), interfaceCode, {
          encoding: 'utf-8',
        })
      }
    } else {
      registerTheme({})
    }
  } catch (error) {
    console.error(error)
    registerTheme({})
  }
  if (!existsSync(options.cssDir))
    await mkdir(options.cssDir, { recursive: true })
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
    await writeDataFiles({
      package: libPackage,
      cssDir,
      devupFile,
      distDir,
    })
    if (!extractCss) return

    api.transform(
      {
        test: cssDir,
      },
      () => globalCss,
    )

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
          css_file,
        } = codeExtract(resourcePath, code, libPackage, cssDir, singleCss)

        if (css) {
          if (globalCss.length < css.length) globalCss = css
          await writeFile(
            join(cssDir, basename(css_file)),
            `/* ${resourcePath} ${Date.now()} */`,
            'utf-8',
          )
        }
        return {
          code: retCode,
          map,
        }
      },
    )
  },
})
