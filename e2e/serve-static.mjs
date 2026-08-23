/**
 * Custom static file server for the landing export.
 * Handles clean URLs by preferring .html files over directories.
 *
 * Serves whichever bundler produced the artifact: vinext writes
 * `apps/landing/dist/client`, the CI-only Next build writes
 * `apps/landing/out`. Override with LANDING_OUTPUT_ROOT (or LANDING_BUILD_MODE
 * = next) so the same suite gates both.
 *
 * Usage: node e2e/serve-static.mjs [port]
 */
import { readFile, stat } from 'node:fs/promises'
import { createServer } from 'node:http'
import { extname, isAbsolute, join, relative, resolve } from 'node:path'

const PORT = parseInt(process.argv[2] || '3099', 10)
const DEFAULT_ROOT =
  process.env.LANDING_BUILD_MODE === 'next'
    ? join('apps', 'landing', 'out')
    : join('apps', 'landing', 'dist', 'client')
const ROOT = resolve(
  process.cwd(),
  process.env.LANDING_OUTPUT_ROOT ?? DEFAULT_ROOT,
)
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

/**
 * Next's segment-cache prefetch asks for a flat, dot-joined payload name
 * (`__next.<key>.<segment>.<segment>.__PAGE__.txt`), while `output: 'export'`
 * writes that payload as a directory tree
 * (`__next.<key>/<segment>/<segment>/__PAGE__.txt`) and leaves the mapping to
 * the host. Resolving it here keeps the CI-only Next artifact under the same
 * "no 404, no console error" assertions as the deployed vinext artifact.
 *
 * Returns undefined for the flat payloads Next writes as real files
 * (`__next._tree.txt`, `__next._index.txt`, `__next._full.txt`,
 * `__next.__PAGE__.txt`), which resolve through the exact-path check.
 */
function toNextSegmentPath(cleanPath) {
  const separator = cleanPath.lastIndexOf('/')
  const directory = separator === -1 ? '' : cleanPath.slice(0, separator + 1)
  const name = cleanPath.slice(separator + 1)

  if (!name.startsWith('__next.') || !name.endsWith('.txt')) return undefined

  const segments = name.slice('__next.'.length, -'.txt'.length).split('.')
  if (segments.length < 2) return undefined

  return `${directory}__next.${segments[0]}/${segments.slice(1).join('/')}.txt`
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

  // 1b. Try the nested layout of a Next segment-cache payload
  const segmentPath = toNextSegmentPath(cleanPath)
  if (segmentPath) {
    const nested = resolve(ROOT, segmentPath)
    if ((await exists(nested)) === 'file') {
      return { filePath: nested, status: 200 }
    }
  }

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
