import { once } from 'node:events'
import { readFile, stat } from 'node:fs/promises'
import { createServer } from 'node:http'
import { extname, isAbsolute, join, relative, resolve } from 'node:path'

async function serve(output) {
  const root = resolve(output)
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
      const types = {
        '.html': 'text/html',
        '.js': 'application/javascript',
        '.css': 'text/css',
      }
      response.writeHead(200, {
        'Content-Type': types[extname(file)] || 'application/octet-stream',
      })
      response.end(await readFile(file))
    } catch (error) {
      console.error(error)
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
      new Promise((resolveClose, reject) => {
        server.closeAllConnections()
        server.close((error) => (error ? reject(error) : resolveClose()))
      }),
  }
}

export async function observe(consumer, tooling) {
  const server = await serve(consumer.output)
  let browser
  const pageErrors = []
  try {
    browser = await tooling.playwright.chromium.launch({ headless: true })
    const page = await browser.newPage()
    page.on('pageerror', (error) => pageErrors.push(error.message))
    const response = await page.goto(server.url, {
      waitUntil: 'networkidle',
      timeout: 30_000,
    })
    if (response.status() !== 200)
      throw new Error(`HTML status ${response.status()}`)
    await page.getByTestId('theme').waitFor()
    await page.getByTestId('imported').waitFor()
    const actual = await page.evaluate(() => {
      function properties(element) {
        if (!element) throw new Error('Missing style target')
        const style = getComputedStyle(element)
        return {
          color: style.color,
          paddingLeft: style.paddingLeft,
          paddingTop: style.paddingTop,
          primary: style.getPropertyValue('--primary').trim(),
          gutter: style.getPropertyValue('--gutter').trim(),
        }
      }
      return {
        theme: properties(document.querySelector('[data-testid="theme"]')),
        imported: properties(
          document.querySelector('[data-testid="imported"]'),
        ),
        root: properties(document.documentElement),
        hrefs: [...document.querySelectorAll('link[rel="stylesheet"]')].map(
          (link) => link.href,
        ),
      }
    })
    const parse = tooling.parse
    const linked = []
    const declarations = []
    for (const href of actual.hrefs) {
      const cssResponse = await page.request.get(href)
      if (cssResponse.status() !== 200)
        throw new Error(`CSS status ${cssResponse.status()}: ${href}`)
      const text = await cssResponse.text()
      linked.push({ href, text })
      // Next bundles PostCSS; parse real declarations rather than matching var() references.
      parse(text).walkDecls((declaration) =>
        declarations.push({
          property: declaration.prop,
          value: declaration.value,
        }),
      )
    }
    return {
      ...actual,
      linked,
      declarations,
      pageErrors,
      browser: browser.version(),
    }
  } finally {
    try {
      if (browser) await browser.close()
    } finally {
      await server.close()
      console.info('extends-theme: owned browser and server closed')
    }
  }
}
