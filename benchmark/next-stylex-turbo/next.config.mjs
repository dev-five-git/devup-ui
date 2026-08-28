import withStylexTurbopack from '@stylexswc/nextjs-plugin/turbopack'

export default withStylexTurbopack({
  rsOptions: {
    dev: process.env.NODE_ENV !== 'production',
  },
})({
  pageExtensions: ['js', 'jsx', 'mdx', 'ts', 'tsx'],
})
