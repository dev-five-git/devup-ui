import { writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { type Compiler } from 'webpack'

export interface DevupUiWebpackPluginOptions {
  package: string
  cssFile: string
}

export class DevupUiWebpackPlugin {
  options: DevupUiWebpackPluginOptions

  constructor(options: Partial<DevupUiWebpackPluginOptions>) {
    const inputOptions = options || {}
    const libPackage = inputOptions.package || '@devup-ui/react'

    this.options = {
      package: libPackage,
      cssFile:
        inputOptions.cssFile ||
        fileURLToPath(import.meta.resolve('./devup-ui.css')),
    }
  }

  apply(compiler: Compiler) {
    // Create an empty CSS file
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
