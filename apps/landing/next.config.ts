import { DevupUI } from '@devup-ui/next-plugin'
import createMDX from '@next/mdx'
import type { NextConfig } from 'next'
import createNextIntlPlugin from 'next-intl/plugin'

const baseConfig: NextConfig & {
  experimental: NonNullable<NextConfig['experimental']> & {
    useTypeScriptCli: true
  }
} = {
  experimental: { useTypeScriptCli: true },
  output: 'export',
  pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],
  reactCompiler: true,
}

// vinext produces the deployed artifact and gets the plugin-free config: its
// MDX and Devup wiring live in `vite.config.ts`. Neither build sets
// `deploymentId`: Next reads it as skew protection and falls back to a full
// page load whenever the host cannot echo the id, which a plain static host
// never can, and vinext's static export already checks each payload against
// the build that wrote the page.
//
// LANDING_BUILD_MODE=next selects the CI-only Next build, which adds the MDX
// loader and `@devup-ui/next-plugin` so this app also gates the Turbopack
// production CSS path. `DevupUI` starts a coordinator when called, so it is
// only invoked on this branch.
export default process.env.LANDING_BUILD_MODE === 'next'
  ? createNextIntlPlugin()(
      createMDX({ extension: /\.mdx?$/ })(
        DevupUI(baseConfig, {
          singleCss: process.env.DEVUP_SINGLE_CSS === '1',
        }),
      ),
    )
  : baseConfig
