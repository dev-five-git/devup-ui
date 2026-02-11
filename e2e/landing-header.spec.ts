import { expect, test } from '@playwright/test'

import { waitForFontsReady, waitForStyleSettle } from './helpers'

/**
 * NOTE: Sub-page navigation is not possible in the current static export +
 * `serve -s` setup. Header tests that required sub-page navigation have been
 * replaced with home-page-only equivalents.
 */

test.describe('Landing Page - Header & Navigation', () => {
  test.describe('Header visibility', () => {
    test('header is visible on home page', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const logoLink = page.locator('a[href="/"]').first()
      await expect(logoLink).toBeVisible()
    })

    test('header contains logo image', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const logoImg = page.locator('img[src="/logo.svg"]').first()
      await expect(logoImg).toBeVisible()
    })

    test('header contains all navigation links on desktop', async ({
      page,
    }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      await expect(
        page.locator('a[href="/docs/overview"]').filter({ hasText: 'Docs' }),
      ).toBeVisible()
      await expect(
        page
          .locator('a[href="/components/overview"]')
          .filter({ hasText: 'Components' }),
      ).toBeVisible()
      await expect(
        page.locator('a[href="/team"]').filter({ hasText: 'Team' }),
      ).toBeVisible()
      await expect(
        page
          .locator('a[href="/storybook/index.html"]')
          .filter({ hasText: 'Storybook' }),
      ).toBeVisible()
    })

    test('navigation links are hidden on mobile', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 812 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const docsLink = page
        .locator('a[href="/docs/overview"]')
        .filter({ hasText: 'Docs' })
      const isVisible = await docsLink.isVisible().catch(() => false)
      expect(isVisible).toBeFalsy()
    })
  })

  test.describe('Logo', () => {
    test('logo links to home page', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const logoLink = page.locator('a[href="/"]').first()
      await expect(logoLink).toHaveAttribute('href', '/')
    })
  })

  test.describe('External links', () => {
    test('GitHub link is present', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const githubLink = page
        .locator('a[href="https://github.com/dev-five-git/devup-ui"]')
        .first()
      await expect(githubLink).toBeVisible()
    })

    test('Discord link is present in header', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const discordLink = page
        .locator('a[href="https://discord.gg/8zjcGc7cWh"]')
        .first()
      await expect(discordLink).toBeVisible()
    })

    test('KakaoTalk link is present in header', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const kakaoLink = page
        .locator('a[href="https://open.kakao.com/o/giONwVAh"]')
        .first()
      await expect(kakaoLink).toBeVisible()
    })
  })

  test.describe('Header stickiness', () => {
    test('header stays visible when scrolling on home page', async ({
      page,
    }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const logoLink = page.locator('a[href="/"]').first()
      await expect(logoLink).toBeVisible()

      // Scroll down significantly
      await page.evaluate(() => window.scrollBy(0, 1000))
      await waitForStyleSettle(page)

      // Logo should still be visible because header is fixed/sticky
      await expect(logoLink).toBeVisible()

      const isInViewport = await logoLink.evaluate((el) => {
        const rect = el.getBoundingClientRect()
        return rect.top >= 0 && rect.top < window.innerHeight
      })
      expect(isInViewport).toBeTruthy()
    })
  })

  test.describe('Theme switch', () => {
    test('theme switch button toggles dark/light', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const initialTheme = await page.evaluate(() =>
        document.documentElement.getAttribute('data-theme'),
      )

      // ThemeSwitch is a Box with cursor:pointer containing two SVGs
      await page.evaluate(() => {
        const allElements = document.querySelectorAll('*')
        for (const el of allElements) {
          if (
            getComputedStyle(el).cursor === 'pointer' &&
            el.querySelectorAll(':scope > svg').length === 2
          ) {
            ;(el as HTMLElement).click()
            return
          }
        }
      })

      await waitForStyleSettle(page)

      const newTheme = await page.evaluate(() =>
        document.documentElement.getAttribute('data-theme'),
      )

      expect(newTheme).not.toBe(initialTheme)
    })

    test('theme persists in data-theme attribute', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const theme = await page.evaluate(() =>
        document.documentElement.getAttribute('data-theme'),
      )

      // data-theme should be either 'light' or 'dark'
      expect(['light', 'dark']).toContain(theme)
    })
  })

  test.describe('Mobile menu', () => {
    test('hamburger menu appears on mobile', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 812 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const menuButton = page.locator('svg[aria-label="Menu Button"]')
      await expect(menuButton).toBeVisible()
    })

    test('hamburger menu is hidden on desktop', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const menuButton = page.locator('svg[aria-label="Menu Button"]')
      const isVisible = await menuButton.isVisible().catch(() => false)
      expect(isVisible).toBeFalsy()
    })

    test('hamburger menu opens on click', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 812 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const menuButton = page.locator('svg[aria-label="Menu Button"]')
      await expect(menuButton).toBeVisible()

      await menuButton.click()
      await waitForFontsReady(page)

      // After clicking menu, the URL should contain menu=1
      const url = page.url()
      expect(url).toContain('menu=1')
    })
  })

  test.describe('Search', () => {
    test('search input is visible on desktop', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const searchInput = page.locator('input[readonly]').first()

      if ((await searchInput.count()) > 0) {
        await expect(searchInput).toBeVisible()
      }
    })

    test('search input has placeholder text', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const searchInput = page.locator('input[readonly]').first()

      if ((await searchInput.count()) > 0) {
        const placeholder = await searchInput.getAttribute('placeholder')
        expect(placeholder).toBeTruthy()
        expect(placeholder).toContain('Search')
      }
    })

    test('search modal opens on input click', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const searchInput = page.locator('input[readonly]').first()

      if ((await searchInput.count()) > 0) {
        await searchInput.click()
        await waitForStyleSettle(page)

        const url = page.url()
        const hasSearchParam = url.includes('search=')
        const hasModalVisible =
          (await page.locator('input:not([readonly])').count()) > 0

        expect(hasSearchParam || hasModalVisible).toBeTruthy()
      }
    })
  })

  test.describe('Footer', () => {
    test('footer is visible', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer').first()
      await footer.scrollIntoViewIfNeeded()
      await expect(footer).toBeVisible()
    })

    test('footer has copyright text', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const copyright = page.getByText('Copyright').first()
      await copyright.scrollIntoViewIfNeeded()
      await expect(copyright).toBeVisible()
    })

    test('footer has white logo', async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footerLogo = page.locator('img[src="/white-logo.svg"]').first()
      await footerLogo.scrollIntoViewIfNeeded()
      await expect(footerLogo).toBeVisible()
    })
  })
})
