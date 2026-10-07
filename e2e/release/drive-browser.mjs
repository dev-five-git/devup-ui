import { chromium, firefox, webkit } from '@playwright/test'

import { checkStyles } from './check-styles.mjs'
import { startStaticServer } from './static-server.mjs'

const engines = { chromium, firefox, webkit }
const server = await startStaticServer(process.argv[2])
try {
  for (const name of (
    process.env.RELEASE_BROWSERS ?? 'chromium,firefox,webkit'
  ).split(',')) {
    const engine = engines[name]
    if (!engine) throw new Error(`Unknown browser ${name}`)
    const browser = await engine.launch({ headless: true })
    try {
      const page = await browser.newPage()
      await checkStyles(page, server.url)
      console.info(`${name}: computed styles and runtime execution passed`)
    } finally {
      await browser.close()
    }
  }
} finally {
  await server.close()
}
