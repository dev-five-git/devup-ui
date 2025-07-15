import { DevupUI } from '@devup-ui/next-plugin'
import createMDX from '@next/mdx'
import remarkGfm from 'remark-gfm'

const withMDX = createMDX({
  options: {
    remarkPlugins: [remarkGfm],
  },
  extension: /\.mdx?$/,
})

export default withMDX(
  DevupUI(
    {
      pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],
      output: 'export',
    },
    { include: ['@devup-ui/components', '@devup-ui/reset-css'] },
  ),
)
