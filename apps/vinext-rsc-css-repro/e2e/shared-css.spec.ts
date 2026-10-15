import { expect, type Page, test } from '@playwright/test'

const ROUTES = ['/notice/', '/admins/', '/users/', '/users/edit/'] as const
const EDIT_ROUTE = '/users/edit/'

function permutations<T>(items: readonly T[]): T[][] {
  if (items.length <= 1) return [[...items]]
  return items.flatMap((item, index) =>
    permutations([...items.slice(0, index), ...items.slice(index + 1)]).map(
      (rest) => [item, ...rest],
    ),
  )
}

/**
 * The users list links to the edit page and so prefetches it, which also loads
 * the CSS its client bundle references. Refuse every request to the edit page
 * except its own document, so a page can only get the header CSS from what the
 * server rendered for it.
 */
async function blockEditPrefetch(page: Page) {
  await page.route('**/users/edit/**', (route) => {
    if (route.request().resourceType() === 'document') return route.continue()
    return route.abort()
  })
}

async function expectHeaderStyled(page: Page, route: string) {
  const header = page.locator('[data-page-title]')
  await expect(header, route).toHaveCount(1)

  const applied = await header.evaluate((element) => {
    const read = (target: Element) => {
      const style = getComputedStyle(target)
      return {
        display: style.display,
        justifyContent: style.justifyContent,
        alignItems: style.alignItems,
        marginBottom: style.marginBottom,
        columnGap: style.columnGap,
        rowGap: style.rowGap,
      }
    }
    const group = element.firstElementChild!
    return { header: read(element), group: read(group) }
  })
  expect(applied.header, `${route} header`).toMatchObject({
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '20px',
  })
  expect(applied.group, `${route} title group`).toMatchObject({
    display: 'flex',
    alignItems: 'center',
    columnGap: '12px',
    rowGap: '12px',
  })

  const layout = await header.evaluate((element) => {
    const bounds = element.getBoundingClientRect()
    const group = element.firstElementChild!.getBoundingClientRect()
    const right = element.lastElementChild!.getBoundingClientRect()
    return { bounds, group, right, hasRight: element.childElementCount > 1 }
  })
  expect(layout.group.left, `${route} title group is at the start`).toBe(
    layout.bounds.left,
  )
  if (layout.hasRight) {
    expect(layout.right.right, `${route} right slot is at the end`).toBe(
      layout.bounds.right,
    )
  }
}

/**
 * Every class the header renders must have a rule in a stylesheet the page
 * really loaded (a `<link>` the browser fetched), whatever the files are
 * called.
 */
async function expectHeaderCssLoaded(page: Page, route: string) {
  const result = await page.locator('[data-page-title]').evaluate((element) => {
    const classes = [
      ...element.classList,
      ...element.firstElementChild!.classList,
    ]
    const linked = [...document.styleSheets].filter(
      (sheet) => sheet.href !== null,
    )
    const collect = (rules: CSSRuleList): string[] =>
      [...rules].flatMap((rule) =>
        rule instanceof CSSStyleRule
          ? [rule.selectorText]
          : 'cssRules' in rule
            ? collect(rule.cssRules as CSSRuleList)
            : [],
      )
    const selectors = linked.flatMap((sheet) => collect(sheet.cssRules))
    return {
      linkedSheets: linked.length,
      missing: classes.filter(
        (name) =>
          !selectors.some((selector) =>
            selector.includes(`.${CSS.escape(name)}`),
          ),
      ),
      classes: classes.length,
    }
  })
  expect(result.classes, `${route} header has atomic classes`).toBeGreaterThan(
    0,
  )
  expect(result.linkedSheets, `${route} loaded stylesheets`).toBeGreaterThan(0)
  expect(result.missing, `${route} classes without a loaded rule`).toEqual([])
}

async function expectRoute(page: Page, route: string) {
  await expectHeaderCssLoaded(page, route)
  await expectHeaderStyled(page, route)
}

test.describe('shared Devup UI header in the static vinext build', () => {
  for (const route of ROUTES) {
    test(`direct entry and reload: ${route}`, async ({ browser }) => {
      const context = await browser.newContext()
      const page = await context.newPage()
      await blockEditPrefetch(page)

      await page.goto(route)
      await expectRoute(page, route)

      await page.reload()
      await expectRoute(page, route)
      await context.close()
    })
  }

  for (const order of permutations(ROUTES)) {
    test(`entered in order ${order.join(' -> ')}`, async ({ browser }) => {
      const context = await browser.newContext()
      const page = await context.newPage()
      await blockEditPrefetch(page)

      for (const route of order) {
        await page.goto(route)
        await expectRoute(page, route)
      }
      await context.close()
    })
  }

  test('the edit page itself renders the same header', async ({ page }) => {
    await page.goto(EDIT_ROUTE)
    await expect(page.getByRole('textbox', { name: 'name' })).toBeVisible()
    await expectRoute(page, EDIT_ROUTE)
  })
})
