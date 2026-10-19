// Measure the fixture layouts actually emitted by Next.js and vinext.
import { existsSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

/** Sum all files, or only CSS files, recursively. Missing directories throw. */
function directoryBytes(directory, cssOnly = false) {
  let bytes = 0
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) bytes += directoryBytes(path, cssOnly)
    else if (!cssOnly || path.endsWith('.css')) bytes += stat.size
  }
  return bytes
}

/** Whole-build bytes include caches/server output; CSS bytes are shipped assets only. */
export function measureOutput(benchmarkDir) {
  // vinext also leaves a vestigial .next stub: keep preferring dist.
  const distDir = join(benchmarkDir, 'dist')
  const hasDist = existsSync(distDir)
  const outputDir = hasDist ? distDir : join(benchmarkDir, '.next')
  const browserDir = join(outputDir, hasDist ? 'client' : 'static')
  return {
    bytes: directoryBytes(outputDir),
    // Next Webpack uses static/css; Turbopack uses static/chunks.
    // An existing empty browser directory counts zero; a missing one throws.
    cssBytes: directoryBytes(browserDir, true),
  }
}
