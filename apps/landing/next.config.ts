import { staticExportDeploymentId } from './static-export-config.js'

export default {
  deploymentId: staticExportDeploymentId,
  output: 'export',
  pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],
  reactCompiler: true,
}
