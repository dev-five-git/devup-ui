// Measure the fixture layouts actually emitted by Next.js and vinext.
import { existsSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

/** What `bytes` counts; benchmark runs, baselines and history entries name it */
export const BYTES_DEFINITION = 'deployable'

// Files a build leaves in its output directory that no deployment uploads.
// Paths are relative to the output directory (`.next` or vinext's `dist`)
// and always use `/`.
const BUILD_ONLY = {
  // Every framework
  any: [
    // TypeScript's incremental type-check cache
    // (https://www.typescriptlang.org/tsconfig/#tsBuildInfoFile)
    /\.tsbuildinfo$/,
    // Source maps: not read to serve a page. Next.js's standalone output
    // (below) leaves the server chunk maps out.
    /\.map$/,
  ],
  // `next start` serves `.next` without these. Next.js's own deployable copy,
  // `output: 'standalone'` ("copies only the necessary files for a production
  // deployment", https://nextjs.org/docs/app/api-reference/config/next-config-js/output),
  // leaves every one of them out of its `.next`.
  next: [
    // Webpack/Turbopack, SWC, image and fetch caches and the `.tsbuildinfo`,
    // which CI keeps between builds instead of deploying
    // (https://nextjs.org/docs/app/guides/ci-build-caching)
    /^cache\//,
    // Turbopack chunks that run during the build (PostCSS, loaders)
    /^build\//,
    // Route types generated for the type check
    // (https://nextjs.org/docs/app/api-reference/config/typescript)
    /^types\//,
    // Build diagnostics and timing traces (`trace`, `trace-build`)
    /^diagnostics\//,
    /^trace(?:-[\w-]+)?$/,
    // Output file tracing records, read when a deployment is packaged
    // (https://nextjs.org/docs/app/api-reference/config/next-config-js/output#automatically-copying-traced-files)
    /\.nft\.json$/,
  ],
  // vinext deploys `dist` (`vinext start` serves dist/client and dist/server)
  vinext: [],
}

/**
 * Whether a file of a build output directory is part of the deployment.
 * `framework` is 'next' or 'vinext'; `path` is relative to the output
 * directory with `/` separators.
 */
export function isDeployed(framework, path) {
  return ![...BUILD_ONLY.any, ...BUILD_ONLY[framework]].some((rule) =>
    rule.test(path),
  )
}

const isCss = (path) => path.endsWith('.css')
const isClientCode = (path) => /\.(?:css|m?js)$/.test(path)

/**
 * Sum the files `include` accepts, recursively; `include` gets the path
 * relative to `directory` with `/` separators. Missing directories throw.
 */
function directoryBytes(directory, include, prefix = '') {
  let bytes = 0
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry)
    const relative = prefix + entry
    const stat = statSync(path)
    if (stat.isDirectory())
      bytes += directoryBytes(path, include, relative + '/')
    else if (include(relative)) bytes += stat.size
  }
  return bytes
}

/**
 * Bytes are the deployable build output: the output directory without what
 * `isDeployed` leaves out (caches, type-check and trace artifacts, source
 * maps). CSS bytes and client bytes are browser-shipped assets only: the CSS,
 * and the JavaScript plus CSS.
 */
export function measureOutput(benchmarkDir) {
  // vinext also leaves a vestigial .next stub: keep preferring dist.
  const distDir = join(benchmarkDir, 'dist')
  const hasDist = existsSync(distDir)
  const framework = hasDist ? 'vinext' : 'next'
  const outputDir = hasDist ? distDir : join(benchmarkDir, '.next')
  const browserDir = join(outputDir, hasDist ? 'client' : 'static')
  return {
    bytes: directoryBytes(outputDir, (path) => isDeployed(framework, path)),
    // Next Webpack uses static/css; Turbopack uses static/chunks.
    // An existing empty browser directory counts zero; a missing one throws.
    cssBytes: directoryBytes(browserDir, isCss),
    clientBytes: directoryBytes(browserDir, isClientCode),
  }
}
