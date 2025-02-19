import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'

import {
  codeExtract,
  getThemeInterface,
  registerTheme,
  setDebug,
} from '@devup-ui/wasm'
import { normalizePath, type PluginOption } from 'vite'

export interface DevupUIPluginOptions {
  package: string
  cssFile: string
  devupPath: string
  interfacePath: string
  extractCss: boolean
  debug: boolean
}

function writeDataFiles(
  options: Omit<DevupUIPluginOptions, 'extractCss' | 'debug'>,
) {
  if (!existsSync(options.interfacePath)) mkdirSync(options.interfacePath)
  if (existsSync(options.devupPath)) {
    registerTheme(
      JSON.parse(readFileSync(options.devupPath, 'utf-8'))?.['theme'],
    )
    const interfaceCode = getThemeInterface(
      options.package,
      'DevupThemeColors',
      'DevupThemeTypography',
      'DevupTheme',
    )
    if (interfaceCode) {
      writeFileSync(join(options.interfacePath, 'theme.d.ts'), interfaceCode, {
        encoding: 'utf-8',
      })
    }
  }
  if (!existsSync(options.cssFile))
    writeFileSync(options.cssFile, '', {
      encoding: 'utf-8',
    })
}

let globalCss = ''

export function DevupUI({
  package: libPackage = '@devup-ui/react',
  devupPath = 'devup.json',
  interfacePath = '.df',
  cssFile = resolve(interfacePath, 'devup-ui.css'),
  extractCss = true,
  debug = false,
}: Partial<DevupUIPluginOptions> = {}): PluginOption {
  setDebug(debug)
  try {
    writeDataFiles({
      package: libPackage,
      cssFile,
      devupPath,
      interfacePath,
    })
  } catch (error) {
    console.error(error)
  }
  return {
    name: 'devup-ui',
    config() {
      return {
        server: {
          watch: {
            ignored: [`!${devupPath}`],
          },
        },
        build: {
          rollupOptions: {
            output: {
              manualChunks(id) {
                // merge devup css files
                if (id.startsWith('devup-ui.css')) {
                  return `devup-ui.css`
                }
              },
            },
          },
        },
      }
    },
    apply() {
      return true
    },
    watchChange(id) {
      if (resolve(id) !== resolve(devupPath)) return
      if (existsSync(devupPath)) {
        try {
          writeDataFiles({
            package: libPackage,
            cssFile,
            devupPath,
            interfacePath,
          })
        } catch (error) {
          console.error(error)
        }
      }
    },
    resolveId(id) {
      if (normalizePath(id) === normalizePath(cssFile)) {
        return 'devup-ui.css?v=' + globalCss.length
      }
    },
    load(id) {
      if (id.split('?')[0] === 'devup-ui.css') return globalCss
    },
    enforce: 'pre',
    async transform(code, id) {
      if (!extractCss) return
      if (id.includes('node_modules')) return
      if (!/\.(tsx|ts|js|mjs|jsx)$/i.test(id)) return

      const { code: retCode, css } = codeExtract(id, code, libPackage, cssFile)

      if (css && globalCss.length < css.length) {
        globalCss = css
        await writeFile(cssFile, `/* ${id} ${Date.now()} */`, {
          encoding: 'utf-8',
        })
      }
      return {
        code: retCode,
      }
    },
  }
}
