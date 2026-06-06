import type { Locator } from '@playwright/test'
import { expect, test } from '@playwright/test'

/** Read the background color of a button's inner bg-bearing <div> (or itself). */
function readInnerBg(locator: Locator): Promise<string> {
  return locator.evaluate((el) => {
    const inner = el.querySelector('div') || el
    return getComputedStyle(inner).backgroundColor
  })
}

/**
 * Hover the link, then poll the inner background until it reaches `expectedHex`.
 *
 * `:hover` styles apply via the pointer, and a CSS transition means the computed
 * color is not final on the next frame. A fixed wait races that transition and
 * flakes; auto-retrying the assertion waits exactly as long as needed.
 */
async function expectHoverBg(
  link: Locator,
  expectedHex: string,
): Promise<void> {
  await link.hover()
  await expect
    .poll(async () => normalizeColor(await readInnerBg(link)), {
      timeout: 5_000,
    })
    .toBe(expectedHex)
}

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

    expect(
      normalizeColor(await readInnerBg(getStartedLink)),
      'Before hover: bg should be $text (#2F2F2F)',
    ).toBe('#2f2f2f')

    // After hover: bg should change to $title (#1A1A1A)
    await expectHoverBg(getStartedLink, '#1a1a1a')
  })

  test('Discord button background changes on hover', async ({ page }) => {
    // Discord "Join our Discord" button has bg="$buttonBlue" and _hover={{ bg: '$buttonBlueHover' }}
    // $buttonBlue = #266CCD, $buttonBlueHover = #1453AC
    const discordLink = page.getByRole('link', { name: /Join our Discord/i })
    await expect(discordLink).toBeVisible()

    expect(normalizeColor(await readInnerBg(discordLink))).toBe('#266ccd')
    await expectHoverBg(discordLink, '#1453ac')
  })

  test('KakaoTalk button background changes on hover', async ({ page }) => {
    // KakaoTalk button: bg="$kakaoButton" _hover={{ bg: '$kakaoButtonHover' }}
    // $kakaoButton = #DE9800, $kakaoButtonHover = #C98900
    const kakaoLink = page.getByRole('link', { name: /Open KakaoTalk/i })
    await expect(kakaoLink).toBeVisible()

    expect(normalizeColor(await readInnerBg(kakaoLink))).toBe('#de9800')
    await expectHoverBg(kakaoLink, '#c98900')
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

    // Before hover should be transparent (rgba(0, 0, 0, 0))
    const bgBefore = await readInnerBg(figmaLink)
    const isTransparent =
      bgBefore === 'rgba(0, 0, 0, 0)' || bgBefore === 'transparent'
    expect(
      isTransparent,
      `Expected transparent before hover, got ${bgBefore}`,
    ).toBeTruthy()

    // After hover should be $menuHover = #F6F4FF
    await expectHoverBg(figmaLink, '#f6f4ff')
  })
})
