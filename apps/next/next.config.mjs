// import type {NextConfig} from "next";
import { DevupUI } from '@devup-ui/next-plugin'

const nextConfig = {
  trailingSlash: true,
  /* config options here */
}

export default DevupUI(nextConfig, {
  include: ['vite-lib-example'],
})
