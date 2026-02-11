import type { Page } from '@playwright/test'

/**
 * Wait for all fonts to be loaded and a rendering frame to complete.
 * Replaces waitForTimeout(1000) after page.goto()
 */
export async function waitForFontsReady(page: Page): Promise<void> {
  await page.evaluate(() => document.fonts.ready)
  // One extra rAF to let the browser paint with loaded fonts
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(resolve)),
  )
}

/**
 * Wait for CSS transitions to settle after a style/theme change.
 * Replaces waitForTimeout(100-300) after theme switches, scroll, evaluate, etc.
 */
export async function waitForStyleSettle(page: Page): Promise<void> {
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(resolve)),
  )
}
