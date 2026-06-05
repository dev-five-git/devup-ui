import type { Page } from '@playwright/test'

/**
 * Wait for all fonts to be loaded and a rendering frame to complete.
 * Replaces waitForTimeout(1000) after page.goto()
 *
 * Falls back to waitForLoadState('load') when JavaScript is disabled
 * (page.evaluate is not available in JS-disabled contexts).
 */
export async function waitForFontsReady(page: Page): Promise<void> {
  // Always wait for the load event first (fires after CSS @font-face is parsed).
  await page.waitForLoadState('load')
  try {
    await page.evaluate(async () => {
      if (!document.fonts) return
      // Explicitly load the webfont weights the snapshots are baselined on.
      // The landing @font-face points at a CDN (jsdelivr); on a SLOW CDN the
      // faces stay pending and `document.fonts.ready` can resolve while text is
      // still painted in a fallback font, shifting layout height and producing
      // flaky full-page/section screenshots. Forcing the loads makes the wait
      // deterministic instead of racing first paint.
      try {
        await Promise.all(
          [
            '400 16px Pretendard',
            '700 16px Pretendard',
            '800 16px Pretendard',
          ].map((font) => document.fonts.load(font)),
        )
      } catch {
        // Ignore individual load failures; fall through to fonts.ready.
      }
      await document.fonts.ready
    })
  } catch {
    // JS disabled (e.g. zero-runtime test) — the load event above is enough.
  }
}

/**
 * Wait for CSS transitions to settle after a style/theme change.
 * Replaces waitForTimeout(100-300) after theme switches, scroll, evaluate, etc.
 *
 * Falls back to waitForLoadState('load') when JavaScript is disabled.
 */
export async function waitForStyleSettle(page: Page): Promise<void> {
  await page.waitForTimeout(100)
}
