import { expect, test } from '@playwright/test'

/**
 * Helper to parse a CSS color value (rgb/rgba/hex) to a normalized hex string.
 * Handles rgb(r, g, b), rgba(r, g, b, a), and hex formats.
 */
function normalizeColor(raw: string): string {
  const trimmed = raw.trim().toLowerCase()

  // Handle hex
  if (trimmed.startsWith('#')) {
    const hex = trimmed.replace('#', '')
    if (hex.length === 3) {
      return `#${hex[0]}${hex[0]}${hex[1]}${hex[1]}${hex[2]}${hex[2]}`
    }
    return `#${hex.substring(0, 6)}`
  }

  // Handle rgb(a)
  const match = trimmed.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/)
  if (match) {
    const r = parseInt(match[1], 10).toString(16).padStart(2, '0')
    const g = parseInt(match[2], 10).toString(16).padStart(2, '0')
    const b = parseInt(match[3], 10).toString(16).padStart(2, '0')
    return `#${r}${g}${b}`
  }

  return trimmed
}

test.describe('Landing Page - Computed Styles', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
  })

  test('body has correct background-color from $footerBg (light)', async ({
    page,
  }) => {
    const bgColor = await page.evaluate(
      () => getComputedStyle(document.body).backgroundColor,
    )
    // $footerBg light = #F4F4F6
    expect(normalizeColor(bgColor)).toBe('#f4f4f6')
  })

  test('body has correct text color from $text (light)', async ({ page }) => {
    const color = await page.evaluate(
      () => getComputedStyle(document.body).color,
    )
    // $text light = #2F2F2F
    expect(normalizeColor(color)).toBe('#2f2f2f')
  })

  test('CSS custom properties are set on :root', async ({ page }) => {
    const cssVars = await page.evaluate(() => {
      const root = document.documentElement
      const style = getComputedStyle(root)
      return {
        primary: style.getPropertyValue('--primary').trim(),
        text: style.getPropertyValue('--text').trim(),
        background: style.getPropertyValue('--background').trim(),
        footerBg: style.getPropertyValue('--footerBg').trim(),
        border: style.getPropertyValue('--border').trim(),
      }
    })

    // At least some CSS variables should be defined
    // devup-ui uses light-dark() so variables may contain the function call
    // or the resolved value. We check they are not empty.
    const definedVars = Object.values(cssVars).filter((v) => v.length > 0)
    expect(
      definedVars.length,
      'Expected CSS custom properties to be set on :root',
    ).toBeGreaterThan(0)
  })

  test('TopBanner has a gradient background', async ({ page }) => {
    // TopBanner is the first major section — a VStack with gradient bg
    // Walk up from "Config" text to find the gradient container
    const gradientBg = await page.evaluate(() => {
      // Find element containing "Zero Config" and walk up to find gradient
      const walker = document.createTreeWalker(
        document.body,
        NodeFilter.SHOW_TEXT,
        {
          acceptNode: (node) =>
            node.textContent?.includes('Config')
              ? NodeFilter.FILTER_ACCEPT
              : NodeFilter.FILTER_REJECT,
        },
      )
      let node = walker.nextNode()
      while (node) {
        let el = node.parentElement
        while (el && el !== document.body) {
          const bg = getComputedStyle(el).backgroundImage
          if (bg.includes('gradient')) return bg
          el = el.parentElement
        }
        node = walker.nextNode()
      }
      return ''
    })

    expect(gradientBg, 'TopBanner should have a gradient background').toContain(
      'gradient',
    )
  })

  test('Feature cards have correct background from $containerBackground', async ({
    page,
  }) => {
    // Feature cards contain "Zero Runtime", "Top Performance", etc.
    const cardBg = await page.evaluate(() => {
      const el = Array.from(document.querySelectorAll('*')).find(
        (el) =>
          el.textContent?.trim() === 'Zero Runtime' && el.tagName !== 'BODY',
      )
      if (!el) return ''
      // Walk up to find the card container
      let parent = el.parentElement
      while (parent && parent !== document.body) {
        const bg = getComputedStyle(parent).backgroundColor
        // Look for non-transparent background
        if (bg && bg !== 'rgba(0, 0, 0, 0)' && bg !== 'transparent') {
          return bg
        }
        parent = parent.parentElement
      }
      return ''
    })

    expect(cardBg, 'Feature card should have a background color').toBeTruthy()
    // $containerBackground light = #FFF
    expect(normalizeColor(cardBg)).toBe('#ffffff')
  })

  test('Footer has $footerBg background', async ({ page }) => {
    const footerBg = await page.evaluate(() => {
      const footer = document.querySelector('footer')
      if (!footer) return ''
      return getComputedStyle(footer).backgroundColor
    })

    expect(footerBg, 'Footer should have a background color').toBeTruthy()
    // $footerBg light = #F4F4F6
    expect(normalizeColor(footerBg)).toBe('#f4f4f6')
  })
})
