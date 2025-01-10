import {type Compiler} from "webpack";
import {fileURLToPath} from "node:url"

export interface DevupUiWebpackPluginOptions {
  package: string;
}

export class DevupUiWebpackPlugin {
  options: DevupUiWebpackPluginOptions;

  constructor(options: Partial<DevupUiWebpackPluginOptions>) {
    const inputOptions = options || {};

    this.options = {
      package: inputOptions.package || "@devup-ui/react"
    }
  }

  apply(compiler: Compiler) {
    compiler.options.experiments.asyncWebAssembly = true
    compiler.options.module.rules.push({
      test: /\.(tsx|ts|js|mjs|jsx)$/,
      exclude: /node_modules/,
      use: [
        {
          loader: fileURLToPath(import.meta.resolve("./loader")),
        },
      ],
    })
  }
}

