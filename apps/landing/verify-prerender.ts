import { statSync } from 'node:fs'
import { resolve } from 'node:path'

import {
  getOutputPath,
  getRscOutputPath,
} from 'vinext/internal/utils/prerender-output-paths'

export class PrerenderArtifactError extends Error {
  constructor(
    readonly route: string,
    readonly path: string,
    readonly reason: 'missing' | 'empty' | 'not-file',
  ) {
    super(`Prerender artifact ${reason}: route ${route}, artifact ${path}`)
    this.name = 'PrerenderArtifactError'
  }
}

function requireArtifact(route: string, path: string): void {
  const stat = statSync(path, { throwIfNoEntry: false })
  if (!stat) throw new PrerenderArtifactError(route, path, 'missing')
  if (!stat.isFile()) throw new PrerenderArtifactError(route, path, 'not-file')
  if (stat.size === 0) throw new PrerenderArtifactError(route, path, 'empty')
}

export function verifyPrerender(
  outputDirectory: string,
  expectedRoutes: readonly string[],
): void {
  // Match landing's output: 'export', trailingSlash: false, empty basePath.
  for (const route of expectedRoutes) {
    requireArtifact(
      route,
      resolve(outputDirectory, getOutputPath(route, false)),
    )
    requireArtifact(
      route,
      resolve(
        outputDirectory,
        getRscOutputPath(route, { mode: 'export', trailingSlash: false }),
      ),
    )
  }
  requireArtifact('/404', resolve(outputDirectory, '404.html'))
}
