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
  cssFile: string
  devupPath: string
  interfacePath: string
  watch: boolean
  debug: boolean
  include: string[]
}

export class DevupUIWebpackPlugin {
  options: DevupUIWebpackPluginOptions

  constructor({
    package: libPackage = '@devup-ui/react',
    devupPath = 'devup.json',
    interfacePath = 'df',
    cssFile = resolve(interfacePath, 'devup-ui.css'),
    watch = false,
    debug = false,
    include = [],
  }: Partial<DevupUIWebpackPluginOptions> = {}) {
    this.options = {
      package: libPackage,
      cssFile,
      devupPath,
      interfacePath,
      watch,
      debug,
      include,
    }
  }

  writeDataFiles() {
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

    if (this.options.watch) {
      writeFileSync(this.options.cssFile, `/* ${Date.now()} */`, {
        encoding: 'utf-8',
      })
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
    // Create an empty CSS file
    if (!existsSync(this.options.cssFile))
      writeFileSync(this.options.cssFile, '', { encoding: 'utf-8' })

    compiler.options.plugins.push(
      new compiler.webpack.DefinePlugin({
        'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(getDefaultTheme()),
      }),
    )
    if (!this.options.watch) {
      compiler.hooks.done.tap('DevupUIWebpackPlugin', (stats) => {
        if (!stats.hasErrors()) {
          // write css file
          writeFileSync(this.options.cssFile, getCss(), { encoding: 'utf-8' })
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
              cssFile: this.options.cssFile,
              sheetFile,
              classMapFile,
              watch: this.options.watch,
            },
          },
        ],
      },
      {
        test: this.options.cssFile,
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
