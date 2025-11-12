import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { stat, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import {
  getCss,
  getDefaultTheme,
  getThemeInterface,
  importClassMap,
  importFileMap,
  importSheet,
  registerTheme,
  setDebug,
} from '@devup-ui/wasm'
import { type Compiler } from 'webpack'

export interface DevupUIWebpackPluginOptions {
  package: string
  cssDir: string
  devupFile: string
  distDir: string
  watch: boolean
  debug: boolean
  include: string[]
  singleCss: boolean
}

export class DevupUIWebpackPlugin {
  options: DevupUIWebpackPluginOptions
  sheetFile: string
  classMapFile: string
  fileMapFile: string

  constructor({
    package: libPackage = '@devup-ui/react',
    devupFile = 'devup.json',
    distDir = 'df',
    cssDir = resolve(distDir, 'devup-ui'),
    watch = false,
    debug = false,
    include = [],
    singleCss = false,
  }: Partial<DevupUIWebpackPluginOptions> = {}) {
    this.options = {
      package: libPackage,
      cssDir,
      devupFile,
      distDir,
      watch,
      debug,
      include,
      singleCss,
    }

    this.sheetFile = join(this.options.distDir, 'sheet.json')
    this.classMapFile = join(this.options.distDir, 'classMap.json')
    this.fileMapFile = join(this.options.distDir, 'fileMap.json')
  }

  writeDataFiles() {
    try {
      const content = existsSync(this.options.devupFile)
        ? readFileSync(this.options.devupFile, 'utf-8')
        : undefined

      if (content) {
        registerTheme(JSON.parse(content)?.['theme'] ?? {})
        const interfaceCode = getThemeInterface(
          this.options.package,
          'CustomColors',
          'DevupThemeTypography',
          'DevupTheme',
        )

        if (interfaceCode) {
          writeFileSync(
            join(this.options.distDir, 'theme.d.ts'),
            interfaceCode,
            {
              encoding: 'utf-8',
            },
          )
        }
      } else {
        registerTheme({})
      }
    } catch (error) {
      console.error(error)
      registerTheme({})
    }
    if (!existsSync(this.options.cssDir))
      mkdirSync(this.options.cssDir, { recursive: true })
    if (this.options.watch)
      writeFileSync(
        join(this.options.cssDir, 'devup-ui.css'),
        getCss(null, false),
      )
  }

  apply(compiler: Compiler) {
    setDebug(this.options.debug)
    const existsDevup = existsSync(this.options.devupFile)
    // read devup.json
    if (!existsSync(this.options.distDir))
      mkdirSync(this.options.distDir, { recursive: true })
    writeFileSync(join(this.options.distDir, '.gitignore'), '*', 'utf-8')

    if (this.options.watch) {
      try {
        // load sheet
        if (existsSync(this.sheetFile))
          importSheet(JSON.parse(readFileSync(this.sheetFile, 'utf-8')))
        if (existsSync(this.classMapFile))
          importClassMap(JSON.parse(readFileSync(this.classMapFile, 'utf-8')))
        if (existsSync(this.fileMapFile))
          importFileMap(JSON.parse(readFileSync(this.fileMapFile, 'utf-8')))
      } catch (error) {
        console.error(error)
        importSheet({})
        importClassMap({})
        importFileMap({})
      }
    }
    this.writeDataFiles()

    if (this.options.watch) {
      let lastModifiedTime: number | null = null
      compiler.hooks.watchRun.tapPromise('DevupUIWebpackPlugin', async () => {
        if (existsDevup) {
          const stats = await stat(this.options.devupFile)

          const modifiedTime = stats.mtimeMs
          if (lastModifiedTime && lastModifiedTime !== modifiedTime)
            this.writeDataFiles()

          lastModifiedTime = modifiedTime
        }
      })
    }
    if (existsDevup)
      compiler.hooks.afterCompile.tap('DevupUIWebpackPlugin', (compilation) => {
        compilation.fileDependencies.add(resolve(this.options.devupFile))
      })

    compiler.options.plugins.push(
      new compiler.webpack.DefinePlugin({
        'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(getDefaultTheme()),
      }),
    )
    if (!this.options.watch) {
      compiler.hooks.done.tapPromise('DevupUIWebpackPlugin', async (stats) => {
        if (!stats.hasErrors()) {
          // write css file
          await writeFile(
            join(this.options.cssDir, 'devup-ui.css'),
            getCss(null, false),
            'utf-8',
          )
        }
      })
    }

    compiler.options.module.rules.push(
      {
        test: /\.(tsx|ts|js|mjs|jsx)$/,
        exclude: new RegExp(
          `(node_modules(?!.*(${['@devup-ui', ...this.options.include]
            .join('|')
            .replaceAll('/', '[\\/\\\\_]')})([\\/\\\\.]|$)))|(.mdx.[tj]sx?$)`,
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
              sheetFile: this.sheetFile,
              classMapFile: this.classMapFile,
              fileMapFile: this.fileMapFile,
              watch: this.options.watch,
              singleCss: this.options.singleCss,
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
