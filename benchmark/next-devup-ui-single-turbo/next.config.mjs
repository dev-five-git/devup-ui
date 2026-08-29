// @ts-check

import { DevupUI } from '@devup-ui/next-plugin'

/** @satisfies {import('next').NextConfig} */
const nextConfig = { experimental: { useTypeScriptCli: true } }

export default DevupUI(nextConfig, { singleCss: true })
