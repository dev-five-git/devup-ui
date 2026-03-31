import type { Page } from '@playwright/test'

const CI_SETTLE_DELAY_MS = 250
const LOCAL_SETTLE_DELAY_MS = 100

function getSettleDelayMs(): number {
  return process.env.CI ? CI_SETTLE_DELAY_MS : LOCAL_SETTLE_DELAY_MS
}

/**
 * Wait for all fonts to be loaded and a rendering frame to complete.
 * Replaces waitForTimeout(1000) after page.goto()
 *
 * Falls back to waitForLoadState('load') when JavaScript is disabled
 * (page.evaluate is not available in JS-disabled contexts).
 */
export async function waitForFontsReady(page: Page): Promise<void> {
  await page.waitForLoadState('load')

  try {
    await page.evaluate(async (settleDelayMs) => {
      const wait = (ms: number) =>
        new Promise<void>((resolve) => window.setTimeout(resolve, ms))
      const nextFrame = () =>
        new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))

      if ('fonts' in document) {
        await document.fonts.ready
      }

      const pendingImages = Array.from(document.images).filter(
        (image) => !image.complete,
      )

      await Promise.all(
        pendingImages.map(
          (image) =>
            new Promise<void>((resolve) => {
              image.addEventListener('load', () => resolve(), {
                once: true,
              })
              image.addEventListener('error', () => resolve(), {
                once: true,
              })
            }),
        ),
      )

      await nextFrame()
      await nextFrame()
      await wait(settleDelayMs)
    }, getSettleDelayMs())
  } catch {
    await page.waitForTimeout(getSettleDelayMs())
  }
}

/**
 * Wait for CSS transitions to settle after a style/theme change.
 * Replaces waitForTimeout(100-300) after theme switches, scroll, evaluate, etc.
 *
 * Falls back to waitForLoadState('load') when JavaScript is disabled.
 */
export async function waitForStyleSettle(page: Page): Promise<void> {
  try {
    await page.waitForFunction(() => {
      return Array.from(
        document.querySelectorAll<HTMLLinkElement>('link[rel="stylesheet"]'),
      ).every((link) => {
        if (!link.href) {
          return true
        }

        const { sheet } = link
        if (!sheet) {
          return false
        }

        try {
          void sheet.cssRules
          return true
        } catch {
          return false
        }
      })
    })
  } catch {
    await page.waitForLoadState('load')
  }

  try {
    await page.evaluate(async (settleDelayMs) => {
      const wait = (ms: number) =>
        new Promise<void>((resolve) => window.setTimeout(resolve, ms))
      const nextFrame = () =>
        new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))

      await nextFrame()
      await nextFrame()
      await wait(settleDelayMs)
    }, getSettleDelayMs())
  } catch {
    await page.waitForTimeout(getSettleDelayMs())
  }
}
