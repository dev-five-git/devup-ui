import { unlinkSync, writeFile, writeFileSync } from 'node:fs'
import { createServer, type IncomingMessage, type Server } from 'node:http'
import { basename, dirname, join, relative } from 'node:path'

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
}

function getFileNumFromCssFile(cssFile: string): number | null {
  if (cssFile.endsWith('devup-ui.css')) return null
  return parseInt(cssFile.split('devup-ui-')[1].split('.')[0])
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

// Extraction tracking for waitForIdle
let activeExtractions = 0
let totalExtractions = 0
let lastCompletedAt = 0
const IDLE_THRESHOLD_MS = 500
const MAX_WAIT_MS = 30_000

function waitForIdle(): Promise<void> {
  const start = Date.now()
  return new Promise((resolve) => {
    const check = () => {
      const now = Date.now()
      if (now - start > MAX_WAIT_MS) {
        // Timeout — return whatever CSS we have
        resolve()
        return
      }
      if (
        totalExtractions > 0 &&
        activeExtractions === 0 &&
        now - lastCompletedAt >= IDLE_THRESHOLD_MS
      ) {
        resolve()
        return
      }
      setTimeout(check, 50)
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
        await waitForIdle()
      }

      res.writeHead(200, { 'Content-Type': 'text/css' })
      res.end(getCss(fileNum ?? null, importMainCss))
      return
    }

    if (req.method === 'POST' && url.pathname === '/extract') {
      activeExtractions++
      try {
        const body = JSON.parse(await readBody(req))
        const { filename, code, resourcePath } = body as {
          filename: string
          code: string
          resourcePath: string
        }

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
            new Promise<void>((resolve, reject) =>
              writeFile(
                join(cssDir, 'devup-ui.css'),
                `${getCss(null, false)}\n/* ${Date.now()} */`,
                'utf-8',
                (err) => (err ? reject(err) : resolve()),
              ),
            ),
          )
        }

        if (result.cssFile) {
          const fileNum = getFileNumFromCssFile(result.cssFile)
          promises.push(
            new Promise<void>((resolve, reject) =>
              writeFile(
                join(cssDir, basename(result.cssFile!)),
                getCss(fileNum, true),
                'utf-8',
                (err) => (err ? reject(err) : resolve()),
              ),
            ),
            new Promise<void>((resolve, reject) =>
              writeFile(sheetFile, exportSheet(), 'utf-8', (err) =>
                err ? reject(err) : resolve(),
              ),
            ),
            new Promise<void>((resolve, reject) =>
              writeFile(classMapFile, exportClassMap(), 'utf-8', (err) =>
                err ? reject(err) : resolve(),
              ),
            ),
            new Promise<void>((resolve, reject) =>
              writeFile(fileMapFile, exportFileMap(), 'utf-8', (err) =>
                err ? reject(err) : resolve(),
              ),
            ),
          )

          // In non-singleCss mode, imports are rewritten from devup-ui-N.css to
          // devup-ui.css?fileNum=N (line 142). Turbopack watches devup-ui.css for
          // all these modules, but above we only write devup-ui-N.css. Without
          // updating devup-ui.css, Turbopack never re-runs the css-loader and
          // new CSS rules are invisible to the browser.
          // When updatedBaseStyle is true, devup-ui.css is already written above.
          if (!singleCss && !result.updatedBaseStyle && result.css != null) {
            promises.push(
              new Promise<void>((resolve, reject) =>
                writeFile(
                  join(cssDir, 'devup-ui.css'),
                  `${getCss(null, false)}\n/* ${Date.now()} */`,
                  'utf-8',
                  (err) => (err ? reject(err) : resolve()),
                ),
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
        activeExtractions--
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

/** @internal Reset coordinator state for testing purposes only */
export const resetCoordinator = () => {
  if (server) {
    server.close()
    server = null
  }
  activeExtractions = 0
  totalExtractions = 0
  lastCompletedAt = 0
}
