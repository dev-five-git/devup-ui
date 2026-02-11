import { expect, test } from '@playwright/test'

import { waitForStyleSettle } from './helpers'

function normalizeColor(raw: string): string {
  const trimmed = raw.trim().toLowerCase()
  if (trimmed.startsWith('#')) {
    const hex = trimmed.replace('#', '')
    if (hex.length === 3) {
      return `#${hex[0]}${hex[0]}${hex[1]}${hex[1]}${hex[2]}${hex[2]}`
    }
    return `#${hex.substring(0, 6)}`
  }
  const match = trimmed.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/)
  if (match) {
    const r = parseInt(match[1], 10).toString(16).padStart(2, '0')
    const g = parseInt(match[2], 10).toString(16).padStart(2, '0')
    const b = parseInt(match[3], 10).toString(16).padStart(2, '0')
    return `#${r}${g}${b}`
  }
  return trimmed
}

test.describe('Landing Page - Interactions', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
  })

  test('GetStarted button background changes on hover', async ({ page }) => {
    // GetStarted button has bg="$text" and _hover={{ bg: '$title' }}
    // $text = #2F2F2F, $title = #1A1A1A
    // GetStartedButton renders as <a> with text "Get started" inside nested divs
    const getStartedLink = page.locator('a', { hasText: 'Get started' }).first()
    await expect(getStartedLink).toBeVisible()

    // Get the inner flex container that has the bg (first div child)
    const bgBefore = await getStartedLink.evaluate((el) => {
      const inner = el.querySelector('div') || el
      return getComputedStyle(inner).backgroundColor
    })

    // Hover
    await getStartedLink.hover()
    await waitForStyleSettle(page)

    const bgAfter = await getStartedLink.evaluate((el) => {
      const inner = el.querySelector('div') || el
      return getComputedStyle(inner).backgroundColor
    })

    expect(
      normalizeColor(bgBefore),
      'Before hover: bg should be $text (#2F2F2F)',
    ).toBe('#2f2f2f')
    expect(
      normalizeColor(bgAfter),
      'After hover: bg should change to $title (#1A1A1A)',
    ).toBe('#1a1a1a')
  })

  test('Discord button background changes on hover', async ({ page }) => {
    // Discord "Join our Discord" button has bg="$buttonBlue" and _hover={{ bg: '$buttonBlueHover' }}
    // $buttonBlue = #266CCD, $buttonBlueHover = #1453AC
    const discordLink = page.getByRole('link', { name: /Join our Discord/i })
    await expect(discordLink).toBeVisible()

    const bgBefore = await discordLink.evaluate((el) => {
      const inner = el.querySelector('div') || el
      return getComputedStyle(inner).backgroundColor
    })

    await discordLink.hover()
    await waitForStyleSettle(page)

    const bgAfter = await discordLink.evaluate((el) => {
      const inner = el.querySelector('div') || el
      return getComputedStyle(inner).backgroundColor
    })

    expect(normalizeColor(bgBefore)).toBe('#266ccd')
    expect(normalizeColor(bgAfter)).toBe('#1453ac')
  })

  test('KakaoTalk button background changes on hover', async ({ page }) => {
    // KakaoTalk button: bg="$kakaoButton" _hover={{ bg: '$kakaoButtonHover' }}
    // $kakaoButton = #DE9800, $kakaoButtonHover = #C98900
    const kakaoLink = page.getByRole('link', { name: /Open KakaoTalk/i })
    await expect(kakaoLink).toBeVisible()

    const bgBefore = await kakaoLink.evaluate((el) => {
      const inner = el.querySelector('div') || el
      return getComputedStyle(inner).backgroundColor
    })

    await kakaoLink.hover()
    await waitForStyleSettle(page)

    const bgAfter = await kakaoLink.evaluate((el) => {
      const inner = el.querySelector('div') || el
      return getComputedStyle(inner).backgroundColor
    })

    expect(normalizeColor(bgBefore)).toBe('#de9800')
    expect(normalizeColor(bgAfter)).toBe('#c98900')
  })

  test('FigmaButton background changes on hover', async ({ page }) => {
    // FigmaButton: _hover={{ bg: '$menuHover' }}
    // Default bg is transparent, $menuHover = #F6F4FF
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    const figmaLink = page.getByRole('link', {
      name: /Go Figma Community/i,
    })
    await expect(figmaLink).toBeVisible()

    const bgBefore = await figmaLink.evaluate((el) => {
      const inner = el.querySelector('div') || el
      return getComputedStyle(inner).backgroundColor
    })

    await figmaLink.hover()
    await waitForStyleSettle(page)

    const bgAfter = await figmaLink.evaluate((el) => {
      const inner = el.querySelector('div') || el
      return getComputedStyle(inner).backgroundColor
    })

    // Before hover should be transparent (rgba(0, 0, 0, 0))
    const isTransparent =
      bgBefore === 'rgba(0, 0, 0, 0)' || bgBefore === 'transparent'
    expect(
      isTransparent,
      `Expected transparent before hover, got ${bgBefore}`,
    ).toBeTruthy()

    // After hover should be $menuHover = #F6F4FF
    expect(normalizeColor(bgAfter)).toBe('#f6f4ff')
  })
})
