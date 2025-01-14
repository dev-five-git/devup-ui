import { existsSync, readFileSync, stat, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { getCss, registerTheme } from '@devup-ui/wasm'
import { type Compiler } from 'webpack'

export interface DevupUIWebpackPluginOptions {
  package: string
  cssFile: string
  devupPath: string
}

export class DevupUIWebpackPlugin {
  options: DevupUIWebpackPluginOptions

  constructor(options: Partial<DevupUIWebpackPluginOptions>) {
    const inputOptions = options || {}
    const libPackage = inputOptions.package || '@devup-ui/react'

    this.options = {
      package: libPackage,
      cssFile:
        inputOptions.cssFile ||
        fileURLToPath(import.meta.resolve('./devup-ui.css')),
      devupPath: inputOptions.devupPath ?? 'devup.json',
    }
  }

  apply(compiler: Compiler) {
    // read devup.json
    if (existsSync(this.options.devupPath)) {
      try {
        const devupTheme = JSON.parse(
          readFileSync(this.options.devupPath, 'utf-8'),
        )?.['theme']
        registerTheme(devupTheme)
      } catch (error) {
        console.error(error)
      }

      let lastModifiedTime: number | null = null

      compiler.hooks.afterCompile.tap(
        'ReloadCssOnDevupChangePlugin',
        (compilation) => {
          compilation.fileDependencies.add(this.options.devupPath)
        },
      )
      compiler.hooks.watchRun.tapAsync(
        'ReloadCssOnDevupChangePlugin',
        (_, callback) => {
          stat(this.options.devupPath, (err, stats) => {
            if (err) {
              console.error(`Error checking ${this.options.devupPath}:`, err)
              return callback()
            }

            const modifiedTime = stats.mtimeMs
            if (lastModifiedTime && lastModifiedTime !== modifiedTime) {
              registerTheme(
                JSON.parse(readFileSync(this.options.devupPath, 'utf-8'))?.[
                  'theme'
                ],
              )
              writeFileSync(this.options.cssFile, getCss(), {
                encoding: 'utf-8',
              })
            }

            lastModifiedTime = modifiedTime
            callback()
          })
        },
      )
    }
    // Create an empty CSS file
    if (!existsSync(this.options.cssFile))
      writeFileSync(this.options.cssFile, '', { encoding: 'utf-8' })
    compiler.options.experiments.asyncWebAssembly = true
    compiler.options.module.rules.push({
      test: /\.(tsx|ts|js|mjs|jsx)$/,
      exclude: /node_modules/,
      use: [
        {
          loader: fileURLToPath(import.meta.resolve('./loader')),
          options: {
            package: this.options.package,
            cssFile: this.options.cssFile,
          },
        },
      ],
    })
  }
}
