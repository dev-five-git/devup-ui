import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

import { inventory, sha } from './W72-real-files.mjs'

export function collectResults(
  { installed, app, cwd, root, home, id },
  result,
) {
  const trees = installed
    ? ['df', 'dist', '.next'].map((dir) => ({
        directory: `apps/${app}/${dir}`,
        files: inventory(join(cwd, dir), root),
      }))
    : []
  result.outputTrees = trees.map((tree) => ({
    ...tree,
    sha256: sha(JSON.stringify(tree.files)),
  }))
  result.devupCss = trees
    .flatMap((tree) => tree.files)
    .filter(
      (file) =>
        /\.css$/.test(file.filename) &&
        (/devup-ui[^/]*\.css$/.test(file.filename) ||
          readFileSync(join(root, file.filename), 'utf8').includes(
            '/*! devup-ui',
          )),
    )
  result.apiProcesses = readdirSync(home)
    .filter((name) => name.startsWith(`${id}-api-`) && name.endsWith('.json'))
    .sort()
    .map((name) => ({
      file: name,
      ...JSON.parse(readFileSync(join(home, name), 'utf8')),
    }))
  result.apiCallCount = result.apiProcesses.reduce(
    (sum, proc) =>
      sum +
      Object.values(proc.metrics).reduce(
        (count, metric) => count + metric.count,
        0,
      ),
    0,
  )
  result.insideDevupMs = result.apiProcesses.reduce(
    (sum, proc) => sum + proc.insideMs,
    0,
  )
  result.insideDevupShareOfBuildWall = result.wallMs
    ? result.insideDevupMs / result.wallMs
    : null
  if (result.status === 'success' && !result.apiCallCount) {
    result.status = 'failed'
    result.instrumentationError = 'No binding API timing was flushed'
    process.exitCode = 1
  }
}
