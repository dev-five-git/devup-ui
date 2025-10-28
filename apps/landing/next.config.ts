import { DevupUI } from '@devup-ui/next-plugin'
import createMDX from '@next/mdx'

const withMDX = createMDX({
  extension: /\.mdx?$/,
})

export default withMDX(
  DevupUI({
    pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],
    output: 'export',
    reactCompiler: true,
    cacheComponents: true,
  }),
)
