import { DevupUI } from '@devup-ui/rsbuild-plugin'
import { defineConfig } from '@rsbuild/core'
import { pluginReact } from '@rsbuild/plugin-react'

export default defineConfig({
  plugins: [
    pluginReact(),
    DevupUI({ singleCss: process.env.RELEASE_SINGLE_CSS === '1' }),
  ],
  source: { entry: { index: './src/main.jsx' } },
  html: { title: 'Release compatibility' },
})
