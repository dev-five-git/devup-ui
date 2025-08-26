import {
  existsSync,
  mkdirSync,
  readFileSync,
  stat,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import {
  getCss,
  getDefaultTheme,
  getThemeInterface,
  importClassMap,
  importSheet,
  registerTheme,
  setDebug,
} from '@devup-ui/wasm'
import { type Compiler } from 'webpack'

export interface DevupUIWebpackPluginOptions {
  package: string
  cssDir: string
  devupPath: string
  interfacePath: string
  watch: boolean
  debug: boolean
  include: string[]
  splitCss: boolean
}

export class DevupUIWebpackPlugin {
  options: DevupUIWebpackPluginOptions

  constructor({
    package: libPackage = '@devup-ui/react',
    devupPath = 'devup.json',
    interfacePath = 'df',
    cssDir = resolve(interfacePath, 'devup-ui'),
    watch = false,
    debug = false,
    include = [],
    splitCss = true,
  }: Partial<DevupUIWebpackPluginOptions> = {}) {
    this.options = {
      package: libPackage,
      cssDir,
      devupPath,
      interfacePath,
      watch,
      debug,
      include,
      splitCss,
    }
  }

  async writeDataFiles() {
    registerTheme(
      JSON.parse(readFileSync(this.options.devupPath, 'utf-8'))?.['theme'],
    )
    const interfaceCode = getThemeInterface(
      this.options.package,
      'DevupThemeColors',
      'DevupThemeTypography',
      'DevupTheme',
    )

    if (interfaceCode) {
      writeFileSync(
        join(this.options.interfacePath, 'theme.d.ts'),
        interfaceCode,
        {
          encoding: 'utf-8',
        },
      )
    }

    if (this.options.watch && !existsSync(this.options.cssDir)) {
      mkdirSync(this.options.cssDir, { recursive: true })
      writeFileSync(join(this.options.cssDir, 'devup-ui.css'), '')
    }
  }

  apply(compiler: Compiler) {
    setDebug(this.options.debug)
    // read devup.json
    const existsDevup = existsSync(this.options.devupPath)

    if (!existsSync(this.options.interfacePath))
      mkdirSync(this.options.interfacePath)

    writeFileSync(join(this.options.interfacePath, '.gitignore'), '*', {
      encoding: 'utf-8',
    })

    const sheetFile = join(this.options.interfacePath, 'sheet.json')
    const classMapFile = join(this.options.interfacePath, 'classMap.json')
    if (this.options.watch) {
      try {
        // load sheet
        if (existsSync(sheetFile))
          importSheet(JSON.parse(readFileSync(sheetFile, 'utf-8')))
        if (existsSync(classMapFile))
          importClassMap(JSON.parse(readFileSync(classMapFile, 'utf-8')))
      } catch (error) {
        console.error(error)
      }
      let lastModifiedTime: number | null = null
      compiler.hooks.watchRun.tapAsync(
        'DevupUIWebpackPlugin',
        (_, callback) => {
          if (existsDevup)
            stat(this.options.devupPath, (err, stats) => {
              if (err) {
                console.error(`Error checking ${this.options.devupPath}:`, err)
                return
              }

              const modifiedTime = stats.mtimeMs
              if (lastModifiedTime && lastModifiedTime !== modifiedTime)
                this.writeDataFiles()

              lastModifiedTime = modifiedTime
            })
          callback()
        },
      )
    }
    if (existsDevup) {
      try {
        this.writeDataFiles()
      } catch (error) {
        console.error(error)
      }
      compiler.hooks.afterCompile.tap('DevupUIWebpackPlugin', (compilation) => {
        compilation.fileDependencies.add(resolve(this.options.devupPath))
      })
    }
    if (!existsSync(this.options.cssDir)) {
      mkdirSync(this.options.cssDir, { recursive: true })
      writeFileSync(join(this.options.cssDir, 'devup-ui.css'), '')
    }

    compiler.options.plugins.push(
      new compiler.webpack.DefinePlugin({
        'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(getDefaultTheme()),
      }),
    )
    if (!this.options.watch) {
      compiler.hooks.done.tap('DevupUIWebpackPlugin', (stats) => {
        if (!stats.hasErrors()) {
          // write css file
          writeFileSync(join(this.options.cssDir, 'devup-ui.css'), getCss(), {
            encoding: 'utf-8',
          })
        }
      })
    }

    compiler.options.module.rules.push(
      {
        test: /\.(tsx|ts|js|mjs|jsx)$/,
        exclude: new RegExp(
          this.options.include.length
            ? `node_modules(?!.*(${this.options.include.join('|').replaceAll('/', '[\\/\\\\]')})([\\/\\\\]|$))`
            : 'node_modules',
        ),
        enforce: 'pre',
        use: [
          {
            loader: createRequire(import.meta.url).resolve(
              '@devup-ui/webpack-plugin/loader',
            ),
            options: {
              package: this.options.package,
              cssDir: this.options.cssDir,
              sheetFile,
              classMapFile,
              watch: this.options.watch,
              splitCss: this.options.splitCss,
            },
          },
        ],
      },
      {
        test: this.options.cssDir,
        enforce: 'pre',
        use: [
          {
            loader: createRequire(import.meta.url).resolve(
              '@devup-ui/webpack-plugin/css-loader',
            ),
            options: {
              watch: this.options.watch,
            },
          },
        ],
      },
    )
  }
}
