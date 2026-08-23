import { DevupUI } from '@devup-ui/next-plugin'
import createMDX from '@next/mdx'
import type { NextConfig } from 'next'

import { STATIC_EXPORT_DEPLOYMENT_ID } from './src/utils/static-export-rsc-transport'

const baseConfig: NextConfig = {
  output: 'export',
  pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],
  reactCompiler: true,
}

// vinext produces the deployed artifact and gets the plugin-free config: its
// MDX and Devup wiring live in `vite.config.ts`. `deploymentId` is what the
// static RSC transport stamps onto the payloads it rewrites.
//
// LANDING_BUILD_MODE=next selects the CI-only Next build, which adds the MDX
// loader and `@devup-ui/next-plugin` so this app also gates the Turbopack
// production CSS path. It deliberately drops `deploymentId`: Next reads it as
// skew protection and falls back to a full page load whenever the host cannot
// echo the id, which a plain static host never can. `DevupUI` starts a
// coordinator when called, so it is only invoked on this branch.
export default process.env.LANDING_BUILD_MODE === 'next'
  ? createMDX({ extension: /\.mdx?$/ })(
      DevupUI(baseConfig, {
        singleCss: process.env.DEVUP_SINGLE_CSS === '1',
      }),
    )
  : { ...baseConfig, deploymentId: STATIC_EXPORT_DEPLOYMENT_ID }
