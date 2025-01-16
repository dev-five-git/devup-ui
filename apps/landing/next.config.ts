import { DevupUI } from '@devup-ui/next-plugin'
import createMDX from '@next/mdx'

const withMDX = createMDX({})

export default withMDX(
  DevupUI(
    {
      pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],
      output: 'export',
    },
    {},
  ),
)
