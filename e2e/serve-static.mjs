/**
 * Custom static file server for Next.js static export.
 * Handles clean URLs by preferring .html files over directories.
 * Usage: node e2e/serve-static.mjs [port]
 */
import { readFile, stat } from 'node:fs/promises'
import { createServer } from 'node:http'
import { extname, join } from 'node:path'

const PORT = parseInt(process.argv[2] || '3099', 10)
const ROOT = join(process.cwd(), 'apps', 'landing', 'out')

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
  // 1. Try exact file path
  const exact = join(ROOT, urlPath)
  if ((await exists(exact)) === 'file') return exact

  // 2. Try with .html extension (clean URLs — PRIORITY over directory)
  const withHtml = join(ROOT, urlPath + '.html')
  if ((await exists(withHtml)) === 'file') return withHtml

  // 3. Try index.html inside directory
  const indexHtml = join(ROOT, urlPath, 'index.html')
  if ((await exists(indexHtml)) === 'file') return indexHtml

  // 4. Fallback to root index.html (SPA fallback)
  return join(ROOT, 'index.html')
}

const server = createServer(async (req, res) => {
  const urlPath = decodeURIComponent(
    new URL(req.url, `http://localhost:${PORT}`).pathname,
  )
  const filePath = await resolveFile(urlPath)
  const ext = extname(filePath)
  const contentType = MIME_TYPES[ext] || 'application/octet-stream'

  try {
    const data = await readFile(filePath)
    res.writeHead(200, { 'Content-Type': contentType })
    res.end(data)
  } catch {
    res.writeHead(404, { 'Content-Type': 'text/plain' })
    res.end('Not Found')
  }
})

server.listen(PORT, () => {
  console.info(`Static server ready on http://localhost:${PORT}`)
})
