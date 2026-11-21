// Opens the web export (`expo export --platform web` wrote dist/) in Chromium
// and checks that react-native-web renders the styles Devup UI compiled: the
// card takes the theme's color and switches with the theme, the input takes
// its focus style through the runtime's `Focused`, and the button, a
// `Pressable`, counts a press.
import { createReadStream, existsSync, statSync } from 'node:fs'
import { createServer } from 'node:http'
import { extname, join, normalize } from 'node:path'

import { chromium } from '@playwright/test'

const root = join(import.meta.dirname, 'dist')
const TYPES = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.json': 'application/json',
  '.png': 'image/png',
}

const server = createServer((request, response) => {
  const path = join(root, normalize(new URL(request.url, 'http://x').pathname))
  if (!path.startsWith(root)) {
    response.writeHead(404).end()
    return
  }
  // A path that is no file is the app's page
  const file =
    existsSync(path) && statSync(path).isFile()
      ? path
      : join(root, 'index.html')
  response.writeHead(200, {
    'content-type': TYPES[extname(file)] ?? 'application/octet-stream',
  })
  createReadStream(file).pipe(response)
})
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
const url = `http://127.0.0.1:${server.address().port}/`

const failures = []
/** Fail with `message` unless `actual` is `expected` */
function expectEqual(actual, expected, message) {
  if (actual !== expected) {
    failures.push(`${message}: expected ${expected}, got ${actual}`)
  }
}

const browser = await chromium.launch()
try {
  const page = await browser.newPage({ colorScheme: 'light' })
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(url)
  const card = page.getByTestId('card')
  const name = page.getByTestId('name')
  const count = page.getByTestId('count')
  await card.waitFor()
  /** The computed style `property` of `locator` */
  const style = (locator, property) =>
    locator.evaluate(
      (element, property) => getComputedStyle(element)[property],
      property,
    )

  expectEqual(
    await style(card, 'backgroundColor'),
    'rgb(255, 255, 255)',
    'the card in the light theme',
  )
  expectEqual(
    await style(count, 'backgroundColor'),
    'rgb(90, 68, 255)',
    'the count button',
  )
  await name.focus()
  expectEqual(
    await style(name, 'borderTopColor'),
    'rgb(90, 68, 255)',
    'the focused input',
  )
  await count.click()
  expectEqual(
    (await count.textContent())?.trim(),
    'Pressed 1 times',
    'the count after a press',
  )
  await page.getByTestId('theme').click()
  expectEqual(
    await style(card, 'backgroundColor'),
    'rgb(31, 31, 46)',
    'the card in the dark theme',
  )
  for (const error of errors) failures.push(`page error: ${error}`)
} finally {
  await browser.close()
  server.close()
}

if (failures.length > 0) {
  console.error(failures.join('\n'))
  process.exit(1)
}
console.info('web: react-native-web renders the compiled styles')
