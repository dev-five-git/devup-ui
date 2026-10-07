import { once } from 'node:events'
import { readFile, stat } from 'node:fs/promises'
import { createServer } from 'node:http'
import { extname, isAbsolute, join, relative, resolve } from 'node:path'

const types = {
  '.html': 'text/html',
  '.js': 'application/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.txt': 'text/plain',
}
export async function startStaticServer(outputRoot) {
  const root = resolve(outputRoot)
  const server = createServer(async (request, response) => {
    const path = resolve(
      root,
      `.${new URL(request.url, 'http://localhost').pathname}`,
    )
    if (
      relative(root, path).startsWith('..') ||
      isAbsolute(relative(root, path))
    ) {
      response.writeHead(403).end()
      return
    }
    try {
      const file = (await stat(path)).isDirectory()
        ? join(path, 'index.html')
        : path
      response.writeHead(200, {
        'Content-Type': types[extname(file)] ?? 'application/octet-stream',
      })
      response.end(await readFile(file))
    } catch (error) {
      if (
        !(error instanceof Error) ||
        !('code' in error) ||
        error.code !== 'ENOENT'
      )
        throw error
      response.writeHead(404).end()
    }
  })
  server.listen(0, '127.0.0.1')
  await once(server, 'listening')
  const address = server.address()
  if (!address || typeof address === 'string')
    throw new Error('Expected TCP address')
  return {
    url: `http://127.0.0.1:${address.port}`,
    close: () =>
      new Promise((resolveClose, reject) =>
        server.close((error) => (error ? reject(error) : resolveClose())),
      ),
  }
}
