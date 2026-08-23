import { STATIC_EXPORT_DEPLOYMENT_ID } from './src/utils/static-export-rsc-transport'

export default {
  deploymentId: STATIC_EXPORT_DEPLOYMENT_ID,
  output: 'export',
  pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],
  reactCompiler: true,
}
