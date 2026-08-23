/**
 * Custom static file server for the vinext client export.
 * Handles clean URLs by preferring .html files over directories.
 * Usage: node e2e/serve-static.mjs [port]
 */
import { readFile, stat } from 'node:fs/promises'
import { createServer } from 'node:http'
import { extname, isAbsolute, join, relative, resolve } from 'node:path'

const PORT = parseInt(process.argv[2] || '3099', 10)
const ROOT = resolve(process.cwd(), 'apps', 'landing', 'dist', 'client')
const NOT_FOUND = join(ROOT, '404.html')

const MIME_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.webp': 'image/webp',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.txt': 'text/plain; charset=utf-8',
  '.rsc': 'text/x-component',
  '.map': 'application/json',
}

async function exists(path) {
  try {
    const s = await stat(path)
    return s.isFile() ? 'file' : s.isDirectory() ? 'dir' : false
  } catch {
    return false
  }
}

async function resolveFile(urlPath) {
  const cleanPath = urlPath.replace(/^\/+/, '')
  const exact = resolve(ROOT, cleanPath)
  const relativePath = relative(ROOT, exact)

  if (relativePath.startsWith('..') || isAbsolute(relativePath)) {
    return { filePath: NOT_FOUND, status: 404 }
  }

  // 1. Try exact file path
  if ((await exists(exact)) === 'file') return { filePath: exact, status: 200 }

  // 2. Try with .html extension (clean URLs — PRIORITY over directory)
  const withHtml = `${exact}.html`
  if ((await exists(withHtml)) === 'file') {
    return { filePath: withHtml, status: 200 }
  }

  // 3. Try index.html inside directory
  const indexHtml = join(exact, 'index.html')
  if ((await exists(indexHtml)) === 'file') {
    return { filePath: indexHtml, status: 200 }
  }

  return { filePath: NOT_FOUND, status: 404 }
}

const server = createServer(async (req, res) => {
  const urlPath = decodeURIComponent(
    new URL(req.url, `http://localhost:${PORT}`).pathname,
  )
  const { filePath, status } = await resolveFile(urlPath)
  const ext = extname(filePath)
  const contentType = MIME_TYPES[ext] || 'application/octet-stream'

  try {
    const data = await readFile(filePath)
    res.writeHead(status, { 'Content-Type': contentType })
    res.end(req.method === 'HEAD' ? undefined : data)
  } catch {
    res.writeHead(404, { 'Content-Type': 'text/plain' })
    res.end('Not Found')
  }
})

server.listen(PORT, () => {
  console.info(`Static server ready on http://localhost:${PORT}`)
})
