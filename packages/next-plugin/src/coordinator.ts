import { existsSync, statSync, writeFile, writeFileSync } from 'node:fs'
import { createServer, type IncomingMessage, type Server } from 'node:http'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  type CompilerSession,
  getFileNumByFilename,
  openCompilerSession,
} from '@devup-ui/plugin-utils'

import { formatPortFile, removeOwnPortFile } from './coordinator-port'
import { elapsedMs, profileStart, reportProfile } from './profile'
import { type DevupWasm, withModuleResolver } from './wasm'

export interface CoordinatorOptions {
  /**
   * The app's output directory. Coordinators are kept per resolved `distDir`:
   * starting one replaces only the previous coordinator of the same app.
   */
  distDir: string
  /** The app's engine, from `openCoordinatorCompiler(distDir, ...)`. */
  wasm: DevupWasm
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
   * Route-reachable source graph closure (cwd-relative POSIX). Used both as the
   * production prewarm target and as the deterministic base-css completion
   * signal. It may include imports Turbopack later erases, but prewarming still
   * extracts those files into the sheet. Empty when no routes are detected or
   * the best-effort graph pass failed, in which case the legacy idle heuristic
   * below is the fallback.
   */
  expectedBaseFiles?: string[]
  /**
   * Files synchronously extracted before a production Turbopack build starts.
   * They seed completion tracking because their atoms already exist in the
   * shared WASM sheet even though their loaders have not POSTed `/extract` yet.
   */
  prewarmedFiles?: string[]
  /**
   * Production outputs extracted before Turbopack starts. A loader that
   * receives byte-identical source can return this result without a second
   * WASM extraction; the shared sheet is already populated.
   */
  prewarmedOutputs?: Map<string, PrewarmedOutput>
  /** Generate transform source maps. Defaults to true for existing callers. */
  sourceMap?: boolean
  /**
   * Idle threshold (ms) for the base-css `/css` wait. Defaults to 2500.
   * FALLBACK ONLY — used when `expectedBaseFiles` is empty (no deterministic
   * signal available). Exposed for tests; the plugin omits it.
   */
  idleThresholdMs?: number
  /**
   * Legacy fail-open window (ms) used when completion was not prewarmed and a
   * graph member never reports. A quiet window cannot prove that Turbopack is
   * finished scheduling loaders, so the production plugin avoids depending on
   * this heuristic by extracting its complete route closure up front. Defaults
   * to 10000. Exposed for tests; the plugin omits it.
   */
  quietMs?: number
  /**
   * Hard timeout (ms) for both the idle and per-bucket waits before failing
   * open. Defaults to 60000. Exposed for tests; the plugin omits it.
   */
  maxWaitMs?: number
  /**
   * Development only: the files the registered theme was read from, and how
   * to register it again. Before each `/css` and `/extract` the coordinator
   * re-registers the theme once one of those files changed, since Next does
   * not reload next.config for devup.json. `reload` returns the new file list.
   */
  theme?: {
    files: string[]
    reload: () => string[]
  }
}

export interface PrewarmedOutput {
  code: string
  cssFile?: string
  map?: string
  source: string
  updatedBaseStyle: boolean
  /** Files the extraction read through the module resolver */
  dependencies?: string[]
}

interface ExtractOutputSnapshot extends Omit<PrewarmedOutput, 'source'> {
  css?: string
}

/** Copy every WASM-backed getter once, then release its Rust allocation. */
export function takeExtractOutput(
  output: ReturnType<DevupWasm['codeExtract']>,
): ExtractOutputSnapshot {
  try {
    return {
      code: output.code,
      css: output.css,
      cssFile: output.cssFile,
      map: output.map,
      updatedBaseStyle: output.updatedBaseStyle,
      dependencies: output.dependencies,
    }
  } finally {
    output.free()
  }
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
/** The content of the write running for each path */
const writingContent = new Map<string, string>()

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
        writingContent.set(path, final)
        writeFile(path, final, 'utf-8', (err) => {
          writingContent.delete(path)
          if (err) reject(err)
          else resolve()
        })
      }),
  )

  writeChain.set(path, next)
  return next
}

/** Wait for every pending serialized write to settle. */
function flushPendingWrites(): Promise<void> {
  return Promise.allSettled([...writeChain.values()]).then(() => undefined)
}

/**
 * Write the newest content of every path whose write is queued or still
 * running, synchronously: `close` runs from `process.on('exit')`, where
 * nothing asynchronous completes. Outside of `exit` the queued writes still
 * run afterwards and write the same newest content again.
 */
function writePendingSync(): void {
  for (const [path, content] of new Map([...writingContent, ...latestContent]))
    writeFileSync(path, content, 'utf-8')
}

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = []
    req.on('data', (chunk: Buffer) => chunks.push(chunk))
    req.on('end', () => resolve(Buffer.concat(chunks).toString('utf-8')))
    req.on('error', reject)
  })
}

interface ThemeWatch {
  files: string[]
  stamp: string
  reload: () => string[]
}

/** What changes when a theme input is edited, added or deleted. */
function themeStamp(files: string[]): string {
  return files
    .map((file) => {
      try {
        const stats = statSync(file)
        return `${stats.mtimeMs}:${stats.size}`
      } catch {
        return 'missing'
      }
    })
    .join('|')
}

/** Register the app's theme again when one of its input files has changed. */
function refreshTheme(state: CoordinatorState): void {
  const { themeWatch } = state
  if (!themeWatch || themeStamp(themeWatch.files) === themeWatch.stamp) return
  // A throwing reload keeps the old stamp, so the next request retries.
  const files = themeWatch.reload()
  state.themeWatch = { ...themeWatch, files, stamp: themeStamp(files) }
}

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
//
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
//
// Each coordinator keeps this tracking in a state of its own, so two apps in
// one process never count each other's extractions.
interface CoordinatorState {
  activeExtractions: number
  totalExtractions: number
  lastCompletedAt: number
  pendingExtractStarts: number
  idleThresholdMs: number
  quietMs: number
  maxWaitMs: number
  extractedFiles: Set<string>
  // Where each file the loaders posted lives, to notice the ones deleted since
  extractedSources: Map<string, string>
  fileNumToBucket: Map<number, string>
  bucketToMembers: Map<string, Set<string>>
  canonicalMap: Record<string, string>
  // Route-reachable runtime files the base sheet must wait for (cwd-relative
  // POSIX). When populated, base-css completion is deterministic; empty falls
  // back to the idle heuristic. See CoordinatorOptions.expectedBaseFiles.
  expectedBaseFiles: Set<string>
  // The files the app's theme was read from, under `next dev`
  themeWatch: ThemeWatch | undefined
}

function createState(options: CoordinatorOptions): CoordinatorState {
  return {
    activeExtractions: 0,
    totalExtractions: 0,
    lastCompletedAt: 0,
    pendingExtractStarts: 0,
    idleThresholdMs: options.idleThresholdMs ?? 2500,
    quietMs: options.quietMs ?? 10_000,
    maxWaitMs: options.maxWaitMs ?? 60_000,
    extractedFiles: new Set(options.prewarmedFiles),
    extractedSources: new Map(),
    fileNumToBucket: new Map(),
    bucketToMembers: buildBucketToMembers(options.canonicalMap),
    canonicalMap: options.canonicalMap,
    expectedBaseFiles: new Set(options.expectedBaseFiles),
    themeWatch: options.theme && {
      ...options.theme,
      stamp: themeStamp(options.theme.files),
    },
  }
}

// Legacy fail-open signal for callers that could not prewarm a deterministic
// file set. It only observes extraction traffic; it is NOT a Turbopack
// compilation-complete signal. Production builds seed `extractedFiles` with
// their prewarmed route closure and therefore do not depend on this path.
function bundlerQuiet(state: CoordinatorState, now: number): boolean {
  return (
    state.totalExtractions > 0 &&
    state.activeExtractions === 0 &&
    state.pendingExtractStarts === 0 &&
    now - state.lastCompletedAt >= state.quietMs
  )
}

function baseFilesComplete(state: CoordinatorState): boolean {
  // Deterministic: the base sheet is complete once every route-reachable file
  // has been extracted by either the production prewarm or `/extract`.
  if (state.expectedBaseFiles.size === 0) return false
  for (const file of state.expectedBaseFiles) {
    if (!state.extractedFiles.has(file)) return false
  }
  return true
}

function waitForBase(state: CoordinatorState): Promise<void> {
  const start = Date.now()
  return new Promise((resolve) => {
    const check = () => {
      const now = Date.now()
      if (now - start > state.maxWaitMs) {
        // Last-resort backstop (see waitForBucket). Never fires on a healthy
        // build: either every expected file extracts, or the idle fallback
        // resolves first.
        resolve()
        return
      }
      // Primary, deterministic path.
      if (baseFilesComplete(state)) {
        resolve()
        return
      }
      // Legacy fail-open for a caller that supplied expected files without
      // prewarming them. The Next plugin's production path completes above.
      if (state.expectedBaseFiles.size > 0 && bundlerQuiet(state, now)) {
        resolve()
        return
      }
      // Fallback ONLY when no deterministic signal exists (no routes detected /
      // pre-pass failed -> expectedBaseFiles empty): the legacy idle heuristic.
      if (
        state.expectedBaseFiles.size === 0 &&
        state.totalExtractions > 0 &&
        state.activeExtractions === 0 &&
        state.pendingExtractStarts === 0 &&
        now - state.lastCompletedAt >= state.idleThresholdMs
      ) {
        resolve()
        return
      }
      setTimeout(check, 25)
    }
    check()
  })
}

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

function waitForBucket(state: CoordinatorState, bucket: string): Promise<void> {
  const { extractedFiles, maxWaitMs } = state
  const members = state.bucketToMembers.get(bucket) ?? new Set([bucket])
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
      const now = Date.now()
      // Legacy fail-open for a caller that did not seed the bucket through
      // prewarming. Quiet time alone cannot distinguish an erased import from
      // a later Turbopack extraction wave; production builds complete via the
      // allExtracted branch above.
      if (bundlerQuiet(state, now)) {
        const missing = [...members].filter((m) => !extractedFiles.has(m))
        console.info(
          `[devup-ui] coordinator: serving bucket "${bucket}" through the legacy quiet fallback; treating unreported member(s) as erased imports: ${missing.join(', ')}`,
        )
        resolve()
        return
      }
      if (now - start > maxWaitMs) {
        // Last-resort backstop only — NOT the primary completion mechanism.
        // Fires only when extraction traffic never pauses for `quietMs`
        // within the whole window (a continuously busy build with a genuinely
        // missing member). Turbopack exposes no compilation-complete hook, so
        // a timer is the only available backstop against hanging forever.
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

// One entry per Next app in this process, keyed by its resolved `distDir`. Each
// app keeps its own engine session, listener and tracking state, so a second
// app never closes the first one's coordinator or reads its sheet. Repeated
// config evaluations of the same app share the entry; it is released when the
// last coordinator started for it closes.
interface CoordinatorApp {
  key: string
  session: CompilerSession | null
  /** The shared engine `session` lives in. */
  engine: DevupWasm | null
  server: Server | null
  refs: number
}

const apps = new Map<string, CoordinatorApp>()

/** @internal The registry key of a `distDir`: absolute and normalized. */
export function coordinatorKey(distDir: string): string {
  const key = resolve(distDir)
  return process.platform === 'win32' ? key.toLowerCase() : key
}

function appFor(distDir: string): CoordinatorApp {
  const key = coordinatorKey(distDir)
  let app = apps.get(key)
  if (!app) {
    app = { key, session: null, engine: null, server: null, refs: 0 }
    apps.set(key, app)
  }
  return app
}

/**
 * The engine of the app at `distDir`: one session of its own on the shared
 * `compiler`, opened on first use and kept until the app's last coordinator
 * closes. Run the app's setup and pass it to `startCoordinator` as `wasm`.
 */
export function openCoordinatorCompiler(
  distDir: string,
  compiler: DevupWasm,
): DevupWasm {
  const app = appFor(distDir)
  if (!app.session || app.engine !== compiler) {
    // A session lives in one engine: a re-evaluation on another engine
    // (`engine` option changed) starts over on that one
    app.session?.close()
    app.session = openCompilerSession(compiler)
    app.engine = compiler
  }
  return withModuleResolver(app.session.compiler)
}

function releaseApp(app: CoordinatorApp): void {
  app.refs--
  // Every coordinator started for the app has closed its own listener by now
  if (app.refs > 0) return
  app.session?.close()
  apps.delete(app.key)
}

export function startCoordinator(options: CoordinatorOptions): {
  close: () => void
} {
  const app = appFor(options.distDir)
  // Next may evaluate its config more than once in the same process. Close the
  // app's previous listener before replacing it so its HTTP server and request
  // closure do not remain live for the rest of the build. Other apps' listeners
  // stay untouched.
  app.server?.close()
  const {
    wasm,
    package: libPackage,
    cssDir,
    singleCss,
    sheetFile,
    classMapFile,
    fileMapFile,
    importAliases,
    coordinatorPortFile,
  } = options
  const {
    codeExtract,
    codeExtractWithoutSourceMap,
    exportClassMap,
    exportFileMap,
    exportSheet,
    getCss,
    removeFile,
  } = wasm
  const prewarmedOutputs = options.prewarmedOutputs ?? new Map()
  const extract =
    options.sourceMap === false ? codeExtractWithoutSourceMap : codeExtract

  const state = createState(options)
  const { extractedFiles, extractedSources, fileNumToBucket } = state

  // A deleted file is never extracted again, so its styles would stay in the
  // sheet; its importers are, once they stop importing it. A repeated
  // extraction (an edit, never the first build) forgets the files gone since.
  const removeDeletedSources = async () => {
    let removed = false
    for (const [filename, resourcePath] of extractedSources) {
      if (existsSync(resourcePath)) continue
      extractedSources.delete(filename)
      if (removeFile(filename)) removed = true
    }
    if (!removed) return
    await Promise.all([
      safeWrite(
        join(cssDir, 'devup-ui.css'),
        `${getCss(null, false)}\n/* ${Date.now()} */`,
      ),
      safeWrite(sheetFile, exportSheet()),
      safeWrite(classMapFile, exportClassMap()),
      safeWrite(fileMapFile, exportFileMap()),
    ])
  }

  const coordinatorServer = createServer(async (req, res) => {
    const url = new URL(req.url ?? '/', `http://${req.headers.host}`)

    if (req.method === 'GET' && url.pathname === '/health') {
      res.writeHead(200, { 'Content-Type': 'text/plain' })
      res.end('ok')
      return
    }

    if (req.method === 'GET' && url.pathname === '/css') {
      const cssStartedAt = profileStart()
      const fileNumParam = url.searchParams.get('fileNum')
      const importMainCss = url.searchParams.get('importMainCss') === 'true'
      const shouldWait = url.searchParams.get('waitForIdle') === 'true'
      const fileNum = fileNumParam != null ? parseInt(fileNumParam) : undefined

      try {
        refreshTheme(state)
      } catch (error) {
        res.writeHead(500, { 'Content-Type': 'text/plain' })
        res.end(error instanceof Error ? error.message : String(error))
        return
      }

      if (shouldWait) {
        if (fileNum != null && fileNumToBucket.has(fileNum)) {
          // Deterministic: block until every member of this collapsed bucket
          // has been extracted, then serve the complete chunk.
          await waitForBucket(state, fileNumToBucket.get(fileNum)!)
        } else {
          // Base css (no fileNum) or a bucket no member has reported yet:
          // wait for the deterministic route-reachable file set (idle fallback
          // only when that set is unavailable).
          await waitForBase(state)
        }
      }

      res.writeHead(200, { 'Content-Type': 'text/css' })
      res.end(getCss(fileNum ?? null, importMainCss))
      reportProfile('coordinator.css', {
        durationMs: elapsedMs(cssStartedAt),
        fileNum,
        waitForIdle: shouldWait,
      })
      return
    }

    if (req.method === 'POST' && url.pathname === '/extract') {
      const requestStartedAt = profileStart()
      // Reserve a "start slot" before yielding on `await readBody`. Without
      // this counter, `waitForIdle` could observe activeExtractions=0 in the
      // window between the request hitting this handler and `activeExtractions++`
      // below — making it falsely conclude the build is idle even though
      // more extractions are imminent.
      state.pendingExtractStarts++
      let promotedToActive = false
      let extractedFilename: string | undefined
      try {
        const bodyStartedAt = profileStart()
        const body = JSON.parse(await readBody(req))
        const bodyDurationMs = elapsedMs(bodyStartedAt)
        state.activeExtractions++
        state.pendingExtractStarts--
        promotedToActive = true
        const { filename, code, resourcePath } = body as {
          filename: string
          code: string
          resourcePath: string
        }
        extractedFilename = filename
        refreshTheme(state)
        const repeated = extractedSources.has(filename)
        extractedSources.set(filename, resourcePath)

        let relCssDir = relative(dirname(resourcePath), cssDir).replaceAll(
          '\\',
          '/',
        )
        if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`

        // The production prewarm exists to make the CSS snapshot complete
        // before Turbopack requests it. The generated CSS is already in that
        // snapshot, so re-running WASM here is pure work in either CSS mode.
        // Require exact source equality because Turbopack may hand a loader
        // code modified by an earlier transform.
        const prewarmed = prewarmedOutputs.get(filename)
        const cacheHit = prewarmed?.source === code
        const extractStartedAt = profileStart()
        const result = cacheHit
          ? prewarmed
          : takeExtractOutput(
              extract(
                filename,
                code,
                libPackage,
                relCssDir,
                singleCss,
                false,
                true,
                importAliases,
              ),
            )
        const extractDurationMs = elapsedMs(extractStartedAt)

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

        const snapshotStartedAt = profileStart()
        let classMapSnapshotBytes: number | undefined
        let classMapSnapshotMs: number | undefined
        let cssSnapshotBytes: number | undefined
        let cssSnapshotMs: number | undefined
        let fileMapSnapshotBytes: number | undefined
        let fileMapSnapshotMs: number | undefined
        let sheetSnapshotBytes: number | undefined
        let sheetSnapshotMs: number | undefined
        const promises: Promise<void>[] = []
        if (repeated) promises.push(removeDeletedSources())

        if (!cacheHit && result.updatedBaseStyle) {
          const cssStartedAt =
            snapshotStartedAt === undefined ? undefined : performance.now()
          const css = `${getCss(null, false)}\n/* ${Date.now()} */`
          const cssDurationMs =
            cssStartedAt === undefined ? undefined : elapsedMs(cssStartedAt)
          if (cssDurationMs !== undefined) {
            cssSnapshotMs = (cssSnapshotMs ?? 0) + cssDurationMs
            cssSnapshotBytes = (cssSnapshotBytes ?? 0) + Buffer.byteLength(css)
          }
          promises.push(safeWrite(join(cssDir, 'devup-ui.css'), css))
        }

        if (result.cssFile) {
          const fileNum = getFileNumByFilename(result.cssFile)
          if (fileNum != null) {
            // Record this bucket's fileNum -> canonical bucket path so /css can
            // wait for the bucket's members before serving it.
            fileNumToBucket.set(
              fileNum,
              state.canonicalMap[filename] ?? filename,
            )
          }
        }

        if (!cacheHit && result.cssFile) {
          const fileNum = getFileNumByFilename(result.cssFile)
          const cssStartedAt =
            snapshotStartedAt === undefined ? undefined : performance.now()
          const css = getCss(fileNum, true)
          const cssDurationMs =
            cssStartedAt === undefined ? undefined : elapsedMs(cssStartedAt)
          if (cssDurationMs !== undefined) {
            cssSnapshotMs = (cssSnapshotMs ?? 0) + cssDurationMs
            cssSnapshotBytes = (cssSnapshotBytes ?? 0) + Buffer.byteLength(css)
          }

          const sheetStartedAt =
            snapshotStartedAt === undefined ? undefined : performance.now()
          const sheet = exportSheet()
          sheetSnapshotMs =
            sheetStartedAt === undefined ? undefined : elapsedMs(sheetStartedAt)
          if (snapshotStartedAt !== undefined) {
            sheetSnapshotBytes = Buffer.byteLength(sheet)
          }

          const classMapStartedAt =
            snapshotStartedAt === undefined ? undefined : performance.now()
          const classMap = exportClassMap()
          classMapSnapshotMs =
            classMapStartedAt === undefined
              ? undefined
              : elapsedMs(classMapStartedAt)
          if (snapshotStartedAt !== undefined) {
            classMapSnapshotBytes = Buffer.byteLength(classMap)
          }

          const fileMapStartedAt =
            snapshotStartedAt === undefined ? undefined : performance.now()
          const fileMap = exportFileMap()
          fileMapSnapshotMs =
            fileMapStartedAt === undefined
              ? undefined
              : elapsedMs(fileMapStartedAt)
          if (snapshotStartedAt !== undefined) {
            fileMapSnapshotBytes = Buffer.byteLength(fileMap)
          }
          promises.push(
            safeWrite(join(cssDir, basename(result.cssFile)), css),
            safeWrite(sheetFile, sheet),
            safeWrite(classMapFile, classMap),
            safeWrite(fileMapFile, fileMap),
          )

          // In non-singleCss mode, imports are rewritten from devup-ui-N.css to
          // devup-ui.css?fileNum=N (line 142). Turbopack watches devup-ui.css for
          // all these modules, but above we only write devup-ui-N.css. Without
          // updating devup-ui.css, Turbopack never re-runs the css-loader and
          // new CSS rules are invisible to the browser.
          // When updatedBaseStyle is true, devup-ui.css is already written above.
          if (!singleCss && !result.updatedBaseStyle && result.css != null) {
            const cssStartedAt =
              snapshotStartedAt === undefined ? undefined : performance.now()
            const baseCss = `${getCss(null, false)}\n/* ${Date.now()} */`
            const cssDurationMs =
              cssStartedAt === undefined ? undefined : elapsedMs(cssStartedAt)
            if (cssDurationMs !== undefined) {
              cssSnapshotMs = (cssSnapshotMs ?? 0) + cssDurationMs
              cssSnapshotBytes =
                (cssSnapshotBytes ?? 0) + Buffer.byteLength(baseCss)
            }
            promises.push(safeWrite(join(cssDir, 'devup-ui.css'), baseCss))
          }
        }

        const snapshotDurationMs = elapsedMs(snapshotStartedAt)
        const writeStartedAt = profileStart()
        await Promise.all(promises)

        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(
          JSON.stringify({
            code: transformedCode,
            map: result.map,
            cssFile: result.cssFile,
            updatedBaseStyle: result.updatedBaseStyle,
            dependencies: result.dependencies,
          }),
        )
        reportProfile('coordinator.extract', {
          bodyMs: bodyDurationMs,
          cacheHit,
          classMapSnapshotBytes,
          classMapSnapshotMs,
          cssSnapshotBytes,
          cssSnapshotMs,
          durationMs: elapsedMs(requestStartedAt),
          extractMs: extractDurationMs,
          fileMapSnapshotBytes,
          fileMapSnapshotMs,
          filename,
          sheetSnapshotBytes,
          sheetSnapshotMs,
          sourceBytes:
            requestStartedAt === undefined
              ? undefined
              : Buffer.byteLength(code),
          scheduledWrites: promises.length,
          snapshotMs: snapshotDurationMs,
          writeMs: elapsedMs(writeStartedAt),
        })
      } catch (error) {
        res.writeHead(500, { 'Content-Type': 'application/json' })
        res.end(
          JSON.stringify({
            error: error instanceof Error ? error.message : String(error),
          }),
        )
      } finally {
        if (promotedToActive) {
          state.activeExtractions--
        } else {
          // readBody/JSON.parse threw before we promoted to active, so the
          // pending slot is still ours to release.
          state.pendingExtractStarts--
        }
        // Mark the file processed (success OR failure) so per-bucket waiters
        // never hang on a file that errored — fail open, like the idle path.
        if (extractedFilename != null) extractedFiles.add(extractedFilename)
        state.totalExtractions++
        state.lastCompletedAt = Date.now()
      }
      return
    }

    res.writeHead(404, { 'Content-Type': 'text/plain' })
    res.end('Not Found')
  })

  app.server = coordinatorServer
  app.refs++
  coordinatorServer.listen(0, '127.0.0.1', () => {
    const addr = coordinatorServer.address()
    if (addr && typeof addr !== 'string') {
      writeFileSync(coordinatorPortFile, formatPortFile(addr.port), 'utf-8')
    }
  })

  let closed = false
  return {
    close: () => {
      coordinatorServer.close()
      if (closed) return
      closed = true
      if (app.server === coordinatorServer) {
        app.server = null
        removeOwnPortFile(coordinatorPortFile)
      }
      // The last CSS and state on disk must hold the final sheet, though
      // `close` cannot await the writes still pending
      writePendingSync()
      releaseApp(app)
    },
  }
}

/** @internal Wait for every pending serialized write to settle. */
export const flushCoordinatorWrites = (): Promise<void> => flushPendingWrites()

/** @internal Reset coordinator state for testing purposes only */
export const resetCoordinator = () => {
  for (const app of apps.values()) {
    app.server?.close()
    app.session?.close()
  }
  apps.clear()
  writeChain.clear()
  latestContent.clear()
  writingContent.clear()
}
