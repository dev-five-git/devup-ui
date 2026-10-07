import { DevupUI } from '@devup-ui/next-plugin'

export default DevupUI(
  {
    output: 'export',
    experimental: { useTypeScriptCli: true },
  },
  {
    singleCss: process.env.RELEASE_SINGLE_CSS === '1',
  },
)
