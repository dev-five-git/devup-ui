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
import { normalizePath, type PluginOption, type UserConfig } from 'vite'

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

async function writeDataFiles(
  options: Omit<
    DevupUIPluginOptions,
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
  if (!existsSync(options.cssDir))
    await mkdir(options.cssDir, { recursive: true })
}

let globalCss = ''

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
      await writeDataFiles({
        package: libPackage,
        cssDir,
        devupFile,
        distDir,
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
          })
        } catch (error) {
          console.error(error)
        }
      }
    },
    resolveId(id) {
      if (
        singleCss &&
        normalizePath(id) === normalizePath(join(cssDir, 'devup-ui.css'))
      )
        return `devup-ui.css?t=${Date.now().toString() + globalCss.length}`
    },
    load(id) {
      if (singleCss && id.split('?')[0] === 'devup-ui.css')
        // for no share env like storybook
        return (globalCss = getCss())
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

      const {
        code: retCode,
        css,
        map,
        css_file,
      } = codeExtract(fileName, code, libPackage, cssDir, singleCss)

      if (css) {
        if (globalCss.length < css.length) globalCss = css
        await writeFile(
          join(cssDir, basename(css_file)),
          `/* ${id} ${Date.now()} */`,
          'utf-8',
        )
      }
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
        bundle[cssFile].source = globalCss
      }
    },
  }
}
