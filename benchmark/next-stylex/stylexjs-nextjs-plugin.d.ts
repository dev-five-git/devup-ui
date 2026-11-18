// @stylexjs/nextjs-plugin ships Flow-typed JavaScript without TypeScript types
declare module '@stylexjs/nextjs-plugin' {
  import type { NextConfig } from 'next'

  export default function stylexPlugin(options: {
    rootDir: string
  }): (nextConfig: NextConfig) => NextConfig
}
