import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, join, resolve } from 'node:path'

import {
  type ImportAliases,
  loadDevupConfig,
  mergeImportAliases,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  registerTheme,
  setDebug,
  setPrefix,
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
  prefix?: string
  /**
   * Import aliases for redirecting imports from other CSS-in-JS libraries
   * Merged with defaults: @emotion/styled, styled-components, @vanilla-extract/css
   * Set to `false` to disable specific aliases
   */
  importAliases?: ImportAliases
}

let globalCss = ''

async function writeDataFiles(
  options: Omit<
    DevupUIRsbuildPluginOptions,
    'extractCss' | 'debug' | 'include'
  >,
) {
  try {
    const config = await loadDevupConfig(options.devupFile)
    const theme = config.theme ?? {}

    registerTheme(theme)
    const interfaceCode = getThemeInterface(
      options.package,
      'CustomColors',
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
  prefix,
  importAliases: userImportAliases,
}: Partial<DevupUIRsbuildPluginOptions> = {}): RsbuildPlugin => {
  const importAliases = mergeImportAliases(userImportAliases)

  return {
    name: 'devup-ui-rsbuild-plugin',
    async setup(api) {
      setDebug(debug)
      if (prefix) {
        setPrefix(prefix)
      }

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
            css = '',
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
            importAliases,
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

          if (cssFile) {
            if (globalCss.length < css.length) globalCss = css
            promises.push(
              writeFile(
                join(cssDir, basename(cssFile)),
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
  }
}
