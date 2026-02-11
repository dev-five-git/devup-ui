import { expect, test } from '@playwright/test'

test.describe('Landing Page - Typography', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
  })

  test('h1 typography: hero text has correct font-weight (800)', async ({
    page,
  }) => {
    // The TopBanner h1 contains "Zero Config" text with typography="h1"
    // h1 at mobile: fontWeight 800, fontSize 38px
    const fontWeight = await page.evaluate(() => {
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
      const node = walker.nextNode()
      if (!node?.parentElement) return ''
      // Walk up to find the element with typography applied
      let el: HTMLElement | null = node.parentElement
      while (el && el !== document.body) {
        const fw = getComputedStyle(el).fontWeight
        if (fw === '800') return fw
        el = el.parentElement
      }
      return getComputedStyle(node.parentElement).fontWeight
    })

    // fontWeight should be 800 (from h1 typography)
    expect(fontWeight).toBe('800')
  })

  test('h4 typography: section headings have font-weight 700', async ({
    page,
  }) => {
    // "Features" heading uses typography="h4" => fontWeight 700
    const fontWeight = await page.evaluate(() => {
      const elements = Array.from(document.querySelectorAll('*'))
      for (const el of elements) {
        if (el.children.length === 0 && el.textContent?.trim() === 'Features') {
          return getComputedStyle(el).fontWeight
        }
      }
      return ''
    })

    expect(fontWeight).toBe('700')
  })

  test('body typography: feature descriptions have font-weight 500', async ({
    page,
  }) => {
    // Feature card descriptions use typography="body" => fontWeight 500
    const fontWeight = await page.evaluate(() => {
      const elements = Array.from(document.querySelectorAll('*'))
      for (const el of elements) {
        if (
          el.children.length === 0 &&
          el.textContent?.includes('futuristic design')
        ) {
          return getComputedStyle(el).fontWeight
        }
      }
      return ''
    })

    expect(fontWeight).toBe('500')
  })

  test('font-family declarations include Pretendard', async ({ page }) => {
    // Check that the body or main text elements have Pretendard in font-family
    const fontFamily = await page.evaluate(() => {
      const el = Array.from(document.querySelectorAll('*')).find(
        (el) =>
          el.children.length === 0 && el.textContent?.trim() === 'Features',
      )
      return el ? getComputedStyle(el).fontFamily : ''
    })

    expect(
      fontFamily.toLowerCase(),
      'Font family should include Pretendard',
    ).toContain('pretendard')
  })

  test('h1 font-size is correct at mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 })
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    // h1 at mobile: fontSize 38px
    const fontSize = await page.evaluate(() => {
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
      const node = walker.nextNode()
      if (!node?.parentElement) return ''
      let el: HTMLElement | null = node.parentElement
      while (el && el !== document.body) {
        const fs = getComputedStyle(el).fontSize
        if (fs === '38px') return fs
        el = el.parentElement
      }
      return getComputedStyle(node.parentElement).fontSize
    })

    expect(fontSize).toBe('38px')
  })

  test('h4 font-size is correct at mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 })
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    // h4 at mobile: fontSize 28px
    const fontSize = await page.evaluate(() => {
      const elements = Array.from(document.querySelectorAll('*'))
      for (const el of elements) {
        if (el.children.length === 0 && el.textContent?.trim() === 'Features') {
          return getComputedStyle(el).fontSize
        }
      }
      return ''
    })

    expect(fontSize).toBe('28px')
  })

  test('letter-spacing is set to -0.03em across typography', async ({
    page,
  }) => {
    // Most typography definitions use letterSpacing -0.03em
    const letterSpacing = await page.evaluate(() => {
      const el = Array.from(document.querySelectorAll('*')).find(
        (el) =>
          el.children.length === 0 && el.textContent?.trim() === 'Features',
      )
      if (!el) return ''
      const fs = parseFloat(getComputedStyle(el).fontSize)
      const ls = parseFloat(getComputedStyle(el).letterSpacing)
      // -0.03em = fontSize * -0.03
      const expected = fs * -0.03
      // Return the ratio for comparison
      return Math.abs(ls - expected) < 0.5 ? 'correct' : `${ls} vs ${expected}`
    })

    expect(letterSpacing).toBe('correct')
  })
})
