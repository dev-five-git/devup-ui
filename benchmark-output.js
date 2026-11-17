// Measure the fixture layouts actually emitted by Next.js and vinext.
import { existsSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

const isCss = (path) => path.endsWith('.css')
const isClientCode = (path) => /\.(?:css|m?js)$/.test(path)

/** Sum the files `include` accepts, recursively. Missing directories throw. */
function directoryBytes(directory, include = () => true) {
  let bytes = 0
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry)
    const stat = statSync(path)
    if (stat.isDirectory()) bytes += directoryBytes(path, include)
    else if (include(path)) bytes += stat.size
  }
  return bytes
}

/**
 * Whole-build bytes include caches/server output. CSS bytes and client bytes
 * are browser-shipped assets only: the CSS, and the JavaScript plus CSS.
 */
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
    cssBytes: directoryBytes(browserDir, isCss),
    clientBytes: directoryBytes(browserDir, isClientCode),
  }
}
