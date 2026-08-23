import type { Page } from '@playwright/test'

const LANDING_FONT_SPECS = [
  '400 16px Pretendard',
  '500 16px Pretendard',
  '600 16px Pretendard',
  '700 16px Pretendard',
  '800 16px Pretendard',
] as const

/**
 * Wait for all fonts to be loaded and a rendering frame to complete.
 * Replaces waitForTimeout(1000) after page.goto()
 *
 * Falls back to waitForLoadState('load') when JavaScript is disabled
 * (page.evaluate is not available in JS-disabled contexts).
 */
export async function waitForFontsReady(page: Page): Promise<void> {
  await page.waitForLoadState('load')

  // With scripting disabled, <noscript> contents are parsed into the DOM and
  // browser-context promises cannot advance. The completed load event is the
  // strongest available signal in those SSR-only contexts.
  if ((await page.locator('noscript iframe').count()) > 0) return

  await page.evaluate(async (fontSpecs) => {
    if (!document.fonts) return

    await Promise.all(fontSpecs.map((font) => document.fonts.load(font)))
    await document.fonts.ready

    const missingFonts = fontSpecs.filter((font) => !document.fonts.check(font))
    if (missingFonts.length > 0) {
      throw new Error(`Required fonts did not load: ${missingFonts.join(', ')}`)
    }

    await Promise.all(
      Array.from(document.images, async (image) => {
        if (!image.currentSrc) return

        if (!image.complete) {
          await new Promise<void>((resolve, reject) => {
            image.addEventListener('load', () => resolve(), { once: true })
            image.addEventListener(
              'error',
              () =>
                reject(new Error(`Image failed to load: ${image.currentSrc}`)),
              { once: true },
            )
          })
        }

        if (image.decode) {
          await image.decode()
        }
      }),
    )

    await new Promise<void>((resolve) => {
      let lastHeight = -1
      let stableFrames = 0

      function checkLayout() {
        const height = document.documentElement.scrollHeight
        stableFrames = height === lastHeight ? stableFrames + 1 : 0
        lastHeight = height

        if (stableFrames >= 2) {
          resolve()
          return
        }

        requestAnimationFrame(checkLayout)
      }

      requestAnimationFrame(checkLayout)
    })
  }, LANDING_FONT_SPECS)
}

/**
 * Wait for CSS transitions to settle after a style/theme change.
 * Replaces waitForTimeout(100-300) after theme switches, scroll, evaluate, etc.
 *
 * Falls back to waitForLoadState('load') when JavaScript is disabled.
 */
export async function waitForStyleSettle(page: Page): Promise<void> {
  await page.waitForLoadState('load')
  if ((await page.locator('noscript iframe').count()) > 0) return

  await page.evaluate(async () => {
    const finiteAnimations = document.getAnimations().filter((animation) => {
      const endTime = animation.effect?.getComputedTiming().endTime
      return typeof endTime === 'number' && Number.isFinite(endTime)
    })

    await Promise.all(
      finiteAnimations.map((animation) =>
        animation.finished.catch(() => undefined),
      ),
    )

    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
    })
  })
}
