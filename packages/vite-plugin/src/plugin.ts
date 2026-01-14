import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

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
import { type PluginOption, type UserConfig } from 'vite'

export interface DevupUIPluginOptions {
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

function getFileNumByFilename(filename: string) {
  if (filename.endsWith('devup-ui.css')) return null
  return parseInt(filename.split('devup-ui-')[1].split('.')[0])
}

async function writeDataFiles(
  options: Omit<DevupUIPluginOptions, 'extractCss' | 'debug' | 'include'>,
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

export function DevupUI({
  package: libPackage = '@devup-ui/react',
  devupFile = 'devup.json',
  distDir = 'df',
  cssDir = resolve(distDir, 'devup-ui'),
  extractCss = true,
  debug = false,
  include = [],
  singleCss = false,
  prefix,
  importAliases: userImportAliases,
}: Partial<DevupUIPluginOptions> = {}): PluginOption {
  setDebug(debug)
  if (prefix) {
    setPrefix(prefix)
  }
  const importAliases = mergeImportAliases(userImportAliases)
  const cssMap = new Map()
  return {
    name: 'devup-ui',
    async configResolved() {
      if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
      await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
      await writeDataFiles({
        package: libPackage,
        cssDir,
        devupFile,
        distDir,
        singleCss,
      })
    },
    config() {
      const theme = getDefaultTheme()
      const define: Record<string, string> = {}
      if (theme) {
        define['process.env.DEVUP_UI_DEFAULT_THEME'] = JSON.stringify(theme)
      }
      const ret: Omit<UserConfig, 'plugins'> = {
        server: {
          watch: {
            ignored: [`!${devupFile}`],
          },
        },
        define,
        optimizeDeps: {
          exclude: [...include, '@devup-ui/components'],
        },
        ssr: {
          noExternal: [...include, /@devup-ui/],
        },
      }
      if (extractCss) {
        ret['build'] = {
          rollupOptions: {
            output: {
              manualChunks(id) {
                // merge devup css files
                const fileName = basename(id).split('?')[0]
                if (/devup-ui(-\d+)?\.css$/.test(fileName)) {
                  return fileName
                }
              },
            },
          },
        }
      }
      return ret
    },
    apply() {
      return true
    },
    async watchChange(id) {
      if (resolve(id) === resolve(devupFile) && existsSync(devupFile)) {
        try {
          await writeDataFiles({
            package: libPackage,
            cssDir,
            devupFile,
            distDir,
            singleCss,
          })
        } catch (error) {
          console.error(error)
        }
      }
    },
    resolveId(id, importer) {
      const fileName = basename(id).split('?')[0]
      if (
        /devup-ui(-\d+)?\.css$/.test(fileName) &&
        resolve(importer ? join(dirname(importer), id) : id) ===
          resolve(join(cssDir, fileName))
      ) {
        return join(
          cssDir,
          `${fileName}?t=${
            Date.now().toString() +
            (cssMap.get(getFileNumByFilename(fileName))?.length ?? 0)
          }`,
        )
      }
    },
    load(id) {
      const fileName = basename(id).split('?')[0]
      if (/devup-ui(-\d+)?\.css$/.test(fileName)) {
        const fileNum = getFileNumByFilename(fileName)
        const css = getCss(fileNum, false)
        cssMap.set(fileNum, css)
        return css
      }
    },
    enforce: 'pre',
    async transform(code, id) {
      if (!extractCss) return

      const fileName = id.split('?')[0]
      if (!/\.(tsx|ts|js|mjs|jsx)$/i.test(fileName)) return
      if (
        new RegExp(
          `node_modules(?!.*(${['@devup-ui', ...include]
            .join('|')
            .replaceAll('/', '[\\/\\\\_]')})([\\/\\\\.]|$))`,
        ).test(fileName)
      ) {
        return
      }

      let rel = relative(dirname(id), cssDir).replaceAll('\\', '/')
      if (!rel.startsWith('./')) rel = `./${rel}`

      const {
        code: retCode,
        css = '',
        map,
        cssFile,
        updatedBaseStyle,
        // import main css in code
      } = codeExtract(
        fileName,
        code,
        libPackage,
        rel,
        singleCss,
        true,
        false,
        importAliases,
      )
      const promises: Promise<void>[] = []

      if (updatedBaseStyle) {
        // update base style
        promises.push(
          writeFile(join(cssDir, 'devup-ui.css'), getCss(null, false), 'utf-8'),
        )
      }

      if (cssFile) {
        const fileNum = getFileNumByFilename(cssFile!)
        const prevCss = cssMap.get(fileNum)
        if (prevCss && prevCss.length < css.length) cssMap.set(fileNum, css)
        promises.push(
          writeFile(
            join(cssDir, basename(cssFile!)),
            `/* ${id} ${Date.now()} */`,
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
    async generateBundle(_options, bundle) {
      if (!extractCss) return

      const cssFile = Object.keys(bundle).find(
        (file) => bundle[file].name === 'devup-ui.css',
      )
      if (cssFile && 'source' in bundle[cssFile]) {
        bundle[cssFile].source = cssMap.get(null)!
      }
    },
  }
}
