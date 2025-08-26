import { mkdir, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'

import { codeExtract } from '@devup-ui/wasm'
import type { RsbuildPlugin } from '@rsbuild/core'

export interface DevupUIRsbuildPluginOptions {
  package: string
  cssFile: string
  devupPath: string
  interfacePath: string
  extractCss: boolean
  debug: boolean
  include: string[]
  splitCss: boolean
}

let globalCss = ''

export const DevupUIRsbuildPlugin = ({
  include = [],
  package: libPackage = '@devup-ui/react',
  extractCss = true,
  interfacePath = 'df',
  cssFile = resolve(interfacePath, 'devup-ui.css'),
  splitCss = true,
}: Partial<DevupUIRsbuildPluginOptions> = {}): RsbuildPlugin => ({
  name: 'devup-ui-rsbuild-plugin',

  async setup(api) {
    if (!extractCss) return
    await mkdir(interfacePath, { recursive: true })
    await writeFile(join(interfacePath, '.gitignore'), '*', {
      encoding: 'utf-8',
    })
    await writeFile(cssFile, '')

    api.transform(
      {
        test: cssFile,
      },
      () => globalCss,
    )

    api.transform(
      {
        test: /\.(tsx|ts|js|mjs|jsx)$/,
      },
      async ({ code, resourcePath }) => {
        if (
          include.length
            ? new RegExp(
                `node_modules(?!(${include
                  .map((i) => `.*${i}`)
                  .join('|')
                  .replaceAll('/', '[\\/\\\\_]')})([\\/\\\\.]|$))`,
              ).test(resourcePath)
            : resourcePath.includes('node_modules')
        )
          return code
        const {
          code: retCode,
          css,
          map,
        } = codeExtract(resourcePath, code, libPackage, cssFile, splitCss)

        if (css && globalCss.length < css.length) {
          globalCss = css
          await writeFile(cssFile, `/* ${resourcePath} ${Date.now()} */`, {
            encoding: 'utf-8',
          })
        }
        return {
          code: retCode,
          map,
        }
      },
    )
  },
})
