import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { DevupUI } from '@devup-ui/next-plugin'
import createMDX from '@next/mdx'

const appRoot = dirname(fileURLToPath(import.meta.url))

const withMDX = createMDX({
  extension: /\.mdx?$/,
})

export default withMDX(
  DevupUI(
    {
      pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],
      output: 'export',
      reactCompiler: true,
    },
    {
      devupFile: join(appRoot, 'devup.json'),
      distDir: join(appRoot, 'df'),
      singleCss: process.env.DEVUP_SINGLE_CSS === '1',
    },
  ),
)
