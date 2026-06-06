import { unlinkSync, writeFile, writeFileSync } from 'node:fs'
import { createServer, type IncomingMessage, type Server } from 'node:http'
import { basename, dirname, join, relative } from 'node:path'

import { getFileNumByFilename } from '@devup-ui/plugin-utils'
import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
} from '@devup-ui/wasm'

export interface CoordinatorOptions {
  package: string
  cssDir: string
  singleCss: boolean
  sheetFile: string
  classMapFile: string
  fileMapFile: string
  importAliases: Record<string, string | null>
  coordinatorPortFile: string
  /**
   * Canonical (single-importer collapse) map: cwd-relative POSIX source path ->
   * its canonical bucket path (or the `@global` sentinel). Used to wait for ALL
   * members of a shared CSS bucket before serving it, instead of guessing
   * completion from idle time. Empty when collapse is disabled.
   */
  canonicalMap: Record<string, string>
  /**
   * Route-reachable runtime source files (cwd-relative POSIX), i.e. exactly the
   * files the bundler will compile and POST to `/extract`. Used to resolve the
   * base-css `/css` wait DETERMINISTICALLY — block until every one of these has
   * been extracted, instead of guessing completion from an idle gap. Comes from
   * `computeFileRoutes` (already type-filtered and orphan-free), so it can never
   * contain a phantom file the bundler skips. Empty when no routes are detected
   * (e.g. pages-router) or the best-effort pre-pass failed, in which case the
   * legacy idle heuristic below is the fallback.
   */
  expectedBaseFiles?: string[]
  /**
   * Idle threshold (ms) for the base-css `/css` wait. Defaults to 2500.
   * FALLBACK ONLY — used when `expectedBaseFiles` is empty (no deterministic
   * signal available). Exposed for tests; the plugin omits it.
   */
  idleThresholdMs?: number
  /**
   * Hard timeout (ms) for both the idle and per-bucket waits before failing
   * open. Defaults to 60000. Exposed for tests; the plugin omits it.
   */
  maxWaitMs?: number
}

// Latest-Wins Coalescing Serializer.
//
// Multiple Turbopack workers may call /extract concurrently, each producing
// CSS for the same target file (especially `devup-ui.css` in singleCss mode
// where every file writes to it). Naive `writeFile` calls run in parallel via
// libuv's thread pool with no completion-order guarantees, so a stale snapshot
// can clobber a fresher one — leaving the on-disk CSS missing rules whose
// class names already landed in the JSX markup.
//
// `safeWrite` solves this with a per-path FIFO chain + content coalescing:
//   1. Each call records the latest content for the path (overwrites earlier).
//   2. The next disk write is chained after the previous one for the same
//      path, guaranteeing serial execution in invocation order.
//   3. When the chained write actually runs, it pulls the most recent content
//      (not the original captured value), so intermediate snapshots between
//      enqueue-time and run-time are coalesced into a single write.
//
// Net effect: race becomes mathematically impossible (single-threaded JS +
// FIFO queue), and total disk IO drops dramatically because N stale snapshots
// for the same file are collapsed into 1 effective write.
const writeChain = new Map<string, Promise<void>>()
const latestContent = new Map<string, string>()

function safeWrite(path: string, content: string): Promise<void> {
  // Always record the most recent content for this path so a queued write
  // picks up the latest snapshot when it runs.
  latestContent.set(path, content)

  // Swallow any prior error solely for chaining purposes — the actual caller
  // that hit the error already saw it via the returned promise, but we must
  // not let one failure poison every subsequent write for this path.
  const prev = (writeChain.get(path) ?? Promise.resolve()).catch(() => {})

  const next = prev.then(
    () =>
      new Promise<void>((resolve, reject) => {
        const final = latestContent.get(path)
        if (final === undefined) {
          // An earlier chained run already consumed the latest content for
          // this path; nothing new to write. Resolve as a no-op.
          resolve()
          return
        }
        latestContent.delete(path)
        writeFile(path, final, 'utf-8', (err) =>
          err ? reject(err) : resolve(),
        )
      }),
  )

  writeChain.set(path, next)
  return next
}

// Best-effort drain of every pending write. Used on coordinator close so the
// build process does not exit with stale files mid-flight.
function flushPendingWrites(): Promise<void> {
  return Promise.allSettled([...writeChain.values()]).then(() => undefined)
}

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = []
    req.on('data', (chunk: Buffer) => chunks.push(chunk))
    req.on('end', () => resolve(Buffer.concat(chunks).toString('utf-8')))
    req.on('error', reject)
  })
}

let server: Server | null = null

// Extraction tracking for waitForIdle.
//
// The CSS loader fetches the live sheet from `/css?waitForIdle=true`. In
// production builds the response of that call is what Turbopack bundles
// for the `devup-ui.css` module — there is no second chance. So waitForIdle
// must NOT resolve while there are still extractions in flight, or that
// have not yet started but will start soon.
//
// We track two complementary signals:
//   * activeExtractions / lastCompletedAt → are extractions currently happening?
//   * pendingExtractStarts → did anyone POST /extract that hasn't progressed
//     to `activeExtractions++` yet (e.g. still inside `await readBody(req)`)?
//
// IDLE_THRESHOLD_MS is increased so an early `/css` request — triggered by
// the first .tsx loader resolving its import graph before the rest of the
// route's files are processed — cannot resolve in the gap between two
// extraction batches. Empirically the gap between Turbopack extraction
// "waves" in a 64-route landing build can exceed the previous 500ms, which
// caused the snapshot to capture only the early routes' styles.
let activeExtractions = 0
let totalExtractions = 0
let lastCompletedAt = 0
let pendingExtractStarts = 0
let idleThresholdMs = 2500
let maxWaitMs = 60_000

function baseFilesComplete(): boolean {
  // Deterministic: the base sheet is complete once every route-reachable runtime
  // file has been extracted. Each `/extract` (success OR failure) adds its file
  // to `extractedFiles`, and `expectedBaseFiles` is phantom-free, so this is a
  // device-independent superset check — no idle gap to guess.
  if (expectedBaseFiles.size === 0) return false
  for (const file of expectedBaseFiles) {
    if (!extractedFiles.has(file)) return false
  }
  return true
}

function waitForBase(): Promise<void> {
  const start = Date.now()
  return new Promise((resolve) => {
    const check = () => {
      const now = Date.now()
      if (now - start > maxWaitMs) {
        // Last-resort backstop (see waitForBucket). Never fires on a healthy
        // build: either every expected file extracts, or the idle fallback
        // resolves first.
        resolve()
        return
      }
      // Primary, deterministic path.
      if (baseFilesComplete()) {
        resolve()
        return
      }
      // Fallback ONLY when no deterministic signal exists (no routes detected /
      // pre-pass failed -> expectedBaseFiles empty): the legacy idle heuristic.
      if (
        expectedBaseFiles.size === 0 &&
        totalExtractions > 0 &&
        activeExtractions === 0 &&
        pendingExtractStarts === 0 &&
        now - lastCompletedAt >= idleThresholdMs
      ) {
        resolve()
        return
      }
      setTimeout(check, 25)
    }
    check()
  })
}

// Per-bucket completion tracking (deterministic replacement for waitForIdle on
// collapsed chunks).
//
// Single-importer collapse merges several source files into ONE shared CSS
// chunk (a "bucket"). That chunk is only complete once EVERY member file has
// been extracted. Turbopack, however, may request the chunk as soon as ONE
// member's import resolves. The old global idle heuristic guessed completion
// and dropped late members' atoms when extraction "waves" exceeded the idle
// threshold (flaky CI rendering). Instead we wait for the bucket's KNOWN
// members (from the canonical map) — no guessing, no extra extraction.
const extractedFiles = new Set<string>()
const fileNumToBucket = new Map<number, string>()
let bucketToMembers = new Map<string, Set<string>>()
let canonicalMapRef: Record<string, string> = {}
// Route-reachable runtime files the base sheet must wait for (cwd-relative
// POSIX). When populated, base-css completion is deterministic; empty falls back
// to the idle heuristic. See CoordinatorOptions.expectedBaseFiles.
let expectedBaseFiles = new Set<string>()

function buildBucketToMembers(
  canonicalMap: Record<string, string>,
): Map<string, Set<string>> {
  const map = new Map<string, Set<string>>()
  for (const [member, bucket] of Object.entries(canonicalMap)) {
    // `@global` files contribute to the base sheet, not a numbered bucket.
    if (bucket === '@global') continue
    let members = map.get(bucket)
    if (!members) {
      // The bucket root is itself a member of its own chunk.
      members = new Set([bucket])
      map.set(bucket, members)
    }
    members.add(member)
  }
  return map
}

function waitForBucket(bucket: string): Promise<void> {
  const members = bucketToMembers.get(bucket) ?? new Set([bucket])
  const start = Date.now()
  return new Promise((resolve) => {
    const check = () => {
      let allExtracted = true
      for (const member of members) {
        if (!extractedFiles.has(member)) {
          allExtracted = false
          break
        }
      }
      if (allExtracted) {
        resolve()
        return
      }
      if (Date.now() - start > maxWaitMs) {
        // Last-resort backstop only — NOT the primary completion mechanism.
        //
        // A bucket's member set comes from the import graph (`canonicalMap`),
        // which now excludes type-only edges (`import type` / `export type`):
        // those are erased by the bundler and never POST /extract, so before
        // the fix they were phantom members that hung this wait until the
        // wall clock expired. With runtime-only members, every member of a
        // REQUESTED bucket is reachable and therefore extracted, so the loop
        // above resolves deterministically and this timer never fires on a
        // healthy build — its duration no longer affects correctness. It stays
        // purely to fail open (serve partial CSS) on a pathological graph
        // mismatch instead of hanging the build forever. Turbopack exposes no
        // compilation-complete hook, so a timer is the only available backstop.
        const missing = [...members].filter((m) => !extractedFiles.has(m))
        console.warn(
          `[devup-ui] coordinator: bucket "${bucket}" not complete after ${maxWaitMs}ms; serving partial CSS (missing: ${missing.join(', ')})`,
        )
        resolve()
        return
      }
      setTimeout(check, 25)
    }
    check()
  })
}

export function startCoordinator(options: CoordinatorOptions): {
  close: () => void
} {
  const {
    package: libPackage,
    cssDir,
    singleCss,
    sheetFile,
    classMapFile,
    fileMapFile,
    importAliases,
    coordinatorPortFile,
  } = options

  idleThresholdMs = options.idleThresholdMs ?? 2500
  maxWaitMs = options.maxWaitMs ?? 60_000
  canonicalMapRef = options.canonicalMap
  bucketToMembers = buildBucketToMembers(options.canonicalMap)
  expectedBaseFiles = new Set(options.expectedBaseFiles ?? [])
  extractedFiles.clear()
  fileNumToBucket.clear()

  server = createServer(async (req, res) => {
    const url = new URL(req.url ?? '/', `http://${req.headers.host}`)

    if (req.method === 'GET' && url.pathname === '/health') {
      res.writeHead(200, { 'Content-Type': 'text/plain' })
      res.end('ok')
      return
    }

    if (req.method === 'GET' && url.pathname === '/css') {
      const fileNumParam = url.searchParams.get('fileNum')
      const importMainCss = url.searchParams.get('importMainCss') === 'true'
      const shouldWait = url.searchParams.get('waitForIdle') === 'true'
      const fileNum = fileNumParam != null ? parseInt(fileNumParam) : undefined

      if (shouldWait) {
        if (fileNum != null && fileNumToBucket.has(fileNum)) {
          // Deterministic: block until every member of this collapsed bucket
          // has been extracted, then serve the complete chunk.
          await waitForBucket(fileNumToBucket.get(fileNum)!)
        } else {
          // Base css (no fileNum) or a bucket no member has reported yet:
          // wait for the deterministic route-reachable file set (idle fallback
          // only when that set is unavailable).
          await waitForBase()
        }
      }

      res.writeHead(200, { 'Content-Type': 'text/css' })
      res.end(getCss(fileNum ?? null, importMainCss))
      return
    }

    if (req.method === 'POST' && url.pathname === '/extract') {
      // Reserve a "start slot" before yielding on `await readBody`. Without
      // this counter, `waitForIdle` could observe activeExtractions=0 in the
      // window between the request hitting this handler and `activeExtractions++`
      // below — making it falsely conclude the build is idle even though
      // more extractions are imminent.
      pendingExtractStarts++
      let promotedToActive = false
      let extractedFilename: string | undefined
      try {
        const body = JSON.parse(await readBody(req))
        activeExtractions++
        pendingExtractStarts--
        promotedToActive = true
        const { filename, code, resourcePath } = body as {
          filename: string
          code: string
          resourcePath: string
        }
        extractedFilename = filename

        let relCssDir = relative(dirname(resourcePath), cssDir).replaceAll(
          '\\',
          '/',
        )
        if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`

        const result = codeExtract(
          filename,
          code,
          libPackage,
          relCssDir,
          singleCss,
          false,
          true,
          importAliases,
        )

        // When singleCss=false, rewrite per-file CSS imports so Turbopack can resolve them.
        // Instead of importing "devup-ui-79.css" (which doesn't exist as a resolvable module),
        // rewrite to "devup-ui.css?fileNum=79" — the placeholder file exists and the query
        // makes each import a unique module for Turbopack.
        let transformedCode = result.code
        if (!singleCss && transformedCode) {
          transformedCode = transformedCode.replace(
            /devup-ui-(\d+)\.css/g,
            'devup-ui.css?fileNum=$1',
          )
        }

        const promises: Promise<void>[] = []

        if (result.updatedBaseStyle) {
          promises.push(
            safeWrite(
              join(cssDir, 'devup-ui.css'),
              `${getCss(null, false)}\n/* ${Date.now()} */`,
            ),
          )
        }

        if (result.cssFile) {
          const fileNum = getFileNumByFilename(result.cssFile)
          if (fileNum != null) {
            // Record this bucket's fileNum -> canonical bucket path so /css can
            // wait for the bucket's members before serving it.
            fileNumToBucket.set(fileNum, canonicalMapRef[filename] ?? filename)
          }
          promises.push(
            safeWrite(
              join(cssDir, basename(result.cssFile)),
              getCss(fileNum, true),
            ),
            safeWrite(sheetFile, exportSheet()),
            safeWrite(classMapFile, exportClassMap()),
            safeWrite(fileMapFile, exportFileMap()),
          )

          // In non-singleCss mode, imports are rewritten from devup-ui-N.css to
          // devup-ui.css?fileNum=N (line 142). Turbopack watches devup-ui.css for
          // all these modules, but above we only write devup-ui-N.css. Without
          // updating devup-ui.css, Turbopack never re-runs the css-loader and
          // new CSS rules are invisible to the browser.
          // When updatedBaseStyle is true, devup-ui.css is already written above.
          if (!singleCss && !result.updatedBaseStyle && result.css != null) {
            promises.push(
              safeWrite(
                join(cssDir, 'devup-ui.css'),
                `${getCss(null, false)}\n/* ${Date.now()} */`,
              ),
            )
          }
        }

        await Promise.all(promises)

        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(
          JSON.stringify({
            code: transformedCode,
            map: result.map,
            cssFile: result.cssFile,
            updatedBaseStyle: result.updatedBaseStyle,
          }),
        )
      } catch (error) {
        res.writeHead(500, { 'Content-Type': 'application/json' })
        res.end(
          JSON.stringify({
            error: error instanceof Error ? error.message : String(error),
          }),
        )
      } finally {
        if (promotedToActive) {
          activeExtractions--
        } else {
          // readBody/JSON.parse threw before we promoted to active, so the
          // pending slot is still ours to release.
          pendingExtractStarts--
        }
        // Mark the file processed (success OR failure) so per-bucket waiters
        // never hang on a file that errored — fail open, like the idle path.
        if (extractedFilename != null) extractedFiles.add(extractedFilename)
        totalExtractions++
        lastCompletedAt = Date.now()
      }
      return
    }

    res.writeHead(404, { 'Content-Type': 'text/plain' })
    res.end('Not Found')
  })

  server.listen(0, '127.0.0.1', () => {
    const addr = server!.address()
    if (addr && typeof addr !== 'string') {
      writeFileSync(coordinatorPortFile, String(addr.port), 'utf-8')
    }
  })

  return {
    close: () => {
      // Fire-and-forget drain of any in-flight serialized writes so the
      // last-written CSS reflects the final sheet state, even though
      // `close` itself returns synchronously (it is invoked from
      // `process.on('exit', ...)` where awaiting is not possible).
      void flushPendingWrites()
      if (server) {
        server.close()
        server = null
        try {
          unlinkSync(coordinatorPortFile)
        } catch {
          // ignore if already deleted
        }
      }
    },
  }
}

/** @internal Wait for every pending serialized write to settle. */
export const flushCoordinatorWrites = (): Promise<void> => flushPendingWrites()

/** @internal Reset coordinator state for testing purposes only */
export const resetCoordinator = () => {
  if (server) {
    server.close()
    server = null
  }
  activeExtractions = 0
  totalExtractions = 0
  lastCompletedAt = 0
  pendingExtractStarts = 0
  idleThresholdMs = 2500
  maxWaitMs = 60_000
  extractedFiles.clear()
  fileNumToBucket.clear()
  bucketToMembers = new Map()
  canonicalMapRef = {}
  expectedBaseFiles = new Set()
  writeChain.clear()
  latestContent.clear()
}
