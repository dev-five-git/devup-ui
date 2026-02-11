import { expect, test } from '@playwright/test'

test.describe('Landing Page - Responsive Layout', () => {
  test.describe('Mobile viewport (375px)', () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 812 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
    })

    test('feature grid is single column on mobile', async ({ page }) => {
      // The Feature section uses Grid with gridTemplateColumns=['1fr', null, '1fr 1fr']
      // On mobile (375px), it should be 1fr (single column)
      const gridColumns = await page.evaluate(() => {
        // Find the grid element by looking for the container with "Zero Runtime" card
        const grids = Array.from(document.querySelectorAll('*'))
        for (const el of grids) {
          const style = getComputedStyle(el)
          if (style.display === 'grid') {
            // Check if this grid contains feature cards
            if (el.textContent?.includes('Zero Runtime')) {
              return style.gridTemplateColumns
            }
          }
        }
        return ''
      })

      // On mobile, should be a single column (one value in gridTemplateColumns)
      const columnCount = gridColumns
        .trim()
        .split(/\s+/)
        .filter((v) => v.length > 0).length
      expect(
        columnCount,
        `Expected 1 column on mobile, got gridTemplateColumns="${gridColumns}"`,
      ).toBe(1)
    })

    test('Discord buttons stack vertically on mobile', async ({ page }) => {
      // The Discord section has flexDirection={['column', null, 'row']}
      const flexDir = await page.evaluate(() => {
        // Find container of KakaoTalk / Discord buttons
        const links = Array.from(document.querySelectorAll('a'))
        const kakaoLink = links.find((a) =>
          a.textContent?.includes('Open KakaoTalk'),
        )
        if (!kakaoLink) return ''
        const parent = kakaoLink.parentElement
        if (!parent) return ''
        return getComputedStyle(parent).flexDirection
      })

      expect(flexDir, 'Discord buttons should stack vertically on mobile').toBe(
        'column',
      )
    })
  })

  test.describe('Desktop viewport (1440px)', () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
    })

    test('feature grid is two columns on desktop', async ({ page }) => {
      const gridColumns = await page.evaluate(() => {
        const grids = Array.from(document.querySelectorAll('*'))
        for (const el of grids) {
          const style = getComputedStyle(el)
          if (style.display === 'grid') {
            if (el.textContent?.includes('Zero Runtime')) {
              return style.gridTemplateColumns
            }
          }
        }
        return ''
      })

      // On desktop, gridTemplateColumns should produce 2 columns ('1fr 1fr')
      const columnCount = gridColumns
        .trim()
        .split(/\s+/)
        .filter((v) => v.length > 0).length
      expect(
        columnCount,
        `Expected 2 columns on desktop, got gridTemplateColumns="${gridColumns}"`,
      ).toBe(2)
    })

    test('Discord buttons are in a row on desktop', async ({ page }) => {
      const flexDir = await page.evaluate(() => {
        const links = Array.from(document.querySelectorAll('a'))
        const kakaoLink = links.find((a) =>
          a.textContent?.includes('Open KakaoTalk'),
        )
        if (!kakaoLink) return ''
        const parent = kakaoLink.parentElement
        if (!parent) return ''
        return getComputedStyle(parent).flexDirection
      })

      expect(flexDir, 'Discord buttons should be in a row on desktop').toBe(
        'row',
      )
    })

    test('footer content is in a row on desktop', async ({ page }) => {
      const flexDir = await page.evaluate(() => {
        const footer = document.querySelector('footer')
        if (!footer) return ''
        // Footer's direct flex child
        const flexChild = footer.querySelector('div')
        if (!flexChild) return ''
        return getComputedStyle(flexChild).flexDirection
      })

      expect(flexDir, 'Footer flex should be row on desktop').toBe('row')
    })
  })

  test.describe('Viewport transition', () => {
    test('feature grid changes columns when resizing', async ({ page }) => {
      // Start mobile
      await page.setViewportSize({ width: 375, height: 812 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const mobileColumns = await page.evaluate(() => {
        const grids = Array.from(document.querySelectorAll('*'))
        for (const el of grids) {
          const style = getComputedStyle(el)
          if (
            style.display === 'grid' &&
            el.textContent?.includes('Zero Runtime')
          ) {
            return style.gridTemplateColumns
              .trim()
              .split(/\s+/)
              .filter((v) => v.length > 0).length
          }
        }
        return 0
      })

      // Resize to desktop
      await page.setViewportSize({ width: 1440, height: 900 })
      // Allow CSS to recompute
      await page.waitForTimeout(300)

      const desktopColumns = await page.evaluate(() => {
        const grids = Array.from(document.querySelectorAll('*'))
        for (const el of grids) {
          const style = getComputedStyle(el)
          if (
            style.display === 'grid' &&
            el.textContent?.includes('Zero Runtime')
          ) {
            return style.gridTemplateColumns
              .trim()
              .split(/\s+/)
              .filter((v) => v.length > 0).length
          }
        }
        return 0
      })

      expect(mobileColumns, 'Mobile should have 1 column').toBe(1)
      expect(desktopColumns, 'Desktop should have 2 columns').toBe(2)
    })
  })
})
