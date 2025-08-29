import { existsSync } from 'node:fs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  codeExtract,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  registerTheme,
  setDebug,
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
}

function getFileNumByFilename(filename: string) {
  if (filename.endsWith('devup-ui.css')) return null
  return parseInt(filename.split('devup-ui-')[1].split('.')[0])
}

async function writeDataFiles(
  options: Omit<DevupUIPluginOptions, 'extractCss' | 'debug' | 'include'>,
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
      ? writeFile(join(options.cssDir, 'devup-ui.css'), getCss())
      : Promise.resolve(),
  ])
}

const cssMap: Map<number | null, string> = new Map()

export function DevupUI({
  package: libPackage = '@devup-ui/react',
  devupFile = 'devup.json',
  distDir = 'df',
  cssDir = resolve(distDir, 'devup-ui'),
  extractCss = true,
  debug = false,
  include = [],
  singleCss = false,
}: Partial<DevupUIPluginOptions> = {}): PluginOption {
  setDebug(debug)
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
          exclude: include,
        },
      }
      if (extractCss) {
        ret['build'] = {
          rollupOptions: {
            output: {
              manualChunks(id) {
                // merge devup css files
                if (singleCss && id.endsWith('devup-ui.css')) {
                  return `devup-ui.css`
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
      if (resolve(id) !== resolve(devupFile)) return
      if (existsSync(devupFile)) {
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
    load(id) {
      const fileName = basename(id).split('?')[0]
      if (fileName.startsWith('devup-ui') && fileName.endsWith('.css')) {
        const fileNum = getFileNumByFilename(id)
        const css = getCss(fileNum)
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
        css,
        map,
        css_file,
      } = codeExtract(fileName, code, libPackage, rel, singleCss)

      if (css) {
        const fileNum = getFileNumByFilename(css_file)
        const prevCss = cssMap.get(fileNum)
        if (prevCss && prevCss.length < css.length) cssMap.set(fileNum, css)
        await writeFile(
          join(cssDir, basename(css_file)),
          `/* ${id} ${Date.now()} */`,
          'utf-8',
        )
      }
      // console.log('transform', retCode)
      return {
        code: retCode,
        map,
      }
    },
    async generateBundle(_options, bundle) {
      if (!extractCss) return

      const cssFile = Object.keys(bundle).find(
        (file) => file.includes('devup-ui') && file.endsWith('.css'),
      )
      if (cssFile && 'source' in bundle[cssFile]) {
        const fileNum = getFileNumByFilename(cssFile)
        const css = cssMap.get(fileNum)
        if (css) bundle[cssFile].source = css
      }
    },
  }
}
