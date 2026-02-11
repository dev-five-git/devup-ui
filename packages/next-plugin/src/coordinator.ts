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
      const fileNum = fileNumParam != null ? parseInt(fileNumParam) : undefined
      res.writeHead(200, { 'Content-Type': 'text/css' })
      res.end(getCss(fileNum ?? null, importMainCss))
      return
    }

    if (req.method === 'POST' && url.pathname === '/extract') {
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

        const promises: Promise<void>[] = []

        if (result.updatedBaseStyle) {
          promises.push(
            new Promise<void>((resolve, reject) =>
              writeFile(
                join(cssDir, 'devup-ui.css'),
                getCss(null, false),
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
        }

        await Promise.all(promises)

        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(
          JSON.stringify({
            code: result.code,
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
}
