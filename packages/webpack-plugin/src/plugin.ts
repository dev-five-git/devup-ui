import {
  existsSync,
  mkdirSync,
  readFileSync,
  stat,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { getCss, getThemeInterface, registerTheme } from '@devup-ui/wasm'
import { type Compiler } from 'webpack'

const _filename = fileURLToPath(import.meta.url)
const _dirname = dirname(_filename)

export interface DevupUIWebpackPluginOptions {
  package: string
  cssFile: string
  devupPath: string
  interfacePath: string
}

export class DevupUIWebpackPlugin {
  options: DevupUIWebpackPluginOptions

  constructor({
    package: libPackage = '@devup-ui/react',
    cssFile = join(_dirname, 'devup-ui.css'),
    devupPath = 'devup.json',
    interfacePath = '.df',
  }: Partial<DevupUIWebpackPluginOptions> = {}) {
    this.options = {
      package: libPackage,
      cssFile,
      devupPath,
      interfacePath,
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
      if (!existsSync(this.options.interfacePath))
        mkdirSync(this.options.interfacePath)
      writeFileSync(
        join(this.options.interfacePath, 'theme.d.ts'),
        interfaceCode,
        {
          encoding: 'utf-8',
        },
      )
    }
    writeFileSync(this.options.cssFile, getCss(), {
      encoding: 'utf-8',
    })
  }

  apply(compiler: Compiler) {
    // read devup.json
    const existsDevup = existsSync(this.options.devupPath)
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

    let lastModifiedTime: number | null = null
    compiler.hooks.watchRun.tapAsync('DevupUIWebpackPlugin', (_, callback) => {
      if (existsDevup)
        stat(this.options.devupPath, (err, stats) => {
          if (err) {
            console.error(`Error checking ${this.options.devupPath}:`, err)
            return
          }

          const modifiedTime = stats.mtimeMs
          if (lastModifiedTime && lastModifiedTime !== modifiedTime) {
            this.writeDataFiles()
          }

          lastModifiedTime = modifiedTime
        })
      callback()
    })
    // Create an empty CSS file
    if (!existsSync(this.options.cssFile)) {
      writeFileSync(this.options.cssFile, '', { encoding: 'utf-8' })
    }

    compiler.options.module.rules.push({
      test: this.options.cssFile,
      use: [
        {
          loader: createRequire(import.meta.url).resolve(
            '@devup-ui/webpack-plugin/css-loader',
          ),
        },
      ],
    })

    compiler.options.module.rules.push({
      test: /\.(tsx|ts|js|mjs|jsx)$/,
      exclude: /node_modules/,
      use: [
        {
          loader: createRequire(import.meta.url).resolve(
            '@devup-ui/webpack-plugin/loader',
          ),
          options: {
            plugin: this,
          },
        },
      ],
    })
  }
}
