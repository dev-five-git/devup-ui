import { join } from 'node:path'

import { DevupUIWebpackPlugin } from '@devup-ui/webpack-plugin'

export default {
  mode: 'production',
  entry: './src/main.jsx',
  resolve: { extensions: ['.jsx', '...'] },
  output: {
    path: join(import.meta.dirname, 'dist'),
    filename: '[name].[contenthash].js',
  },
  experiments: { css: true },
  optimization: { minimize: false },
  module: {
    rules: [
      { test: /\.jsx$/, use: [join(import.meta.dirname, 'jsx-loader.cjs')] },
      { test: /\.css$/, type: 'css' },
    ],
  },
  plugins: [
    new DevupUIWebpackPlugin({
      singleCss: process.env.RELEASE_SINGLE_CSS === '1',
    }),
  ],
}
