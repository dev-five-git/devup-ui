const { DevupUIWebpackPlugin } = require('@devup-ui/webpack-plugin')
const { join } = require('node:path')

module.exports = {
  mode: 'production',
  entry: './src/main.jsx',
  resolve: { extensions: ['.jsx', '...'] },
  output: {
    path: join(__dirname, 'dist'),
    filename: '[name].[contenthash].js',
  },
  experiments: { css: true },
  optimization: { minimize: false },
  module: {
    rules: [
      { test: /\.jsx$/, use: [join(__dirname, 'jsx-loader.cjs')] },
      { test: /\.css$/, type: 'css' },
    ],
  },
  plugins: [
    new DevupUIWebpackPlugin({
      singleCss: process.env.RELEASE_SINGLE_CSS === '1',
    }),
  ],
}
