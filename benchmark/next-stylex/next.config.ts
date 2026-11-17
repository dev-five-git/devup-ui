import stylexPlugin from '@stylexjs/nextjs-plugin'

const nextConfig = {
  experimental: { useTypeScriptCli: true },
  // Configure `pageExtensions` to include MDX files
  pageExtensions: ['js', 'jsx', 'mdx', 'ts', 'tsx'],
  transpilePackages: ['@stylexjs/open-props'],
  // Optionally, add any other Next.js config below
}

export default stylexPlugin({
  rootDir: __dirname,
})(nextConfig)
