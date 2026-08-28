const path = require('node:path')

module.exports = {
  presets: ['next/babel'],
  plugins: [
    [
      '@stylexjs/babel-plugin',
      {
        dev: process.env.NODE_ENV !== 'production',
        runtimeInjection: false,
        genConditionalClasses: true,
        treeshakeCompensation: true,
        aliases: {
          '@/*': [path.join(__dirname, '*')],
        },
        unstable_moduleResolution: {
          type: 'commonJS',
        },
      },
    ],
  ],
}
