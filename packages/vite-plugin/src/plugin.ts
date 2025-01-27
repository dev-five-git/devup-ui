import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  codeExtract,
  getCss,
  getThemeInterface,
  registerTheme,
} from '@devup-ui/wasm'
import { type PluginOption } from 'vite'

const _filename = fileURLToPath(import.meta.url)
const _dirname = dirname(_filename)

export interface DevupUIPluginOptions {
  package: string
  cssFile: string
  devupPath: string
  interfacePath: string
  extractCss: boolean
}

function writeDataFiles(options: Omit<DevupUIPluginOptions, 'extractCss'>) {
  registerTheme(JSON.parse(readFileSync(options.devupPath, 'utf-8'))?.['theme'])
  const interfaceCode = getThemeInterface(
    options.package,
    'DevupThemeColors',
    'DevupThemeTypography',
    'DevupTheme',
  )
  if (interfaceCode) {
    if (!existsSync(options.interfacePath)) mkdirSync(options.interfacePath)
    writeFileSync(join(options.interfacePath, 'theme.d.ts'), interfaceCode, {
      encoding: 'utf-8',
    })
  }
  writeFileSync(options.cssFile, getCss(), {
    encoding: 'utf-8',
  })
}

export function DevupUI({
  package: libPackage = '@devup-ui/react',
  cssFile = join(_dirname, 'devup-ui.css'),
  devupPath = 'devup.json',
  interfacePath = '.df',
  extractCss = true,
}: Partial<DevupUIPluginOptions> = {}): PluginOption {
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
  return {
    name: 'devup-ui',
    config() {
      return {
        server: {
          watch: {
            ignored: [`!${devupPath}`],
          },
        },
      }
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
    enforce: 'pre',
    transform(code, id) {
      if (!extractCss) return
      if (id.includes('node_modules')) return
      if (!/\.(tsx|ts|js|mjs|jsx)$/i.test(id)) return

      const { code: retCode, css } = codeExtract(id, code, libPackage, cssFile)

      if (css) {
        // should be reset css
        writeFileSync(cssFile, css, {
          encoding: 'utf-8',
        })
        return {
          code: `${retCode}
            const exists = !!document.getElementById('devup-ui');
            const style = document.getElementById('devup-ui') || document.createElement('style');
            style.id = 'devup-ui';
            style.textContent = \`
            ${css}
            \`;
            if (!exists) document.head.appendChild(style);
          `,
        }
      }
      return {
        code: retCode,
      }
    },
  }
}
