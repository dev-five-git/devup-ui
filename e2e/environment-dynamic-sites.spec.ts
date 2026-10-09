import { expect, test } from '@playwright/test'

import { dynamicModes, renderDynamicSite } from './environment-dynamic-driver'

const parameters = `({ pad, id, children }: { readonly pad: string | undefined; readonly id: string; readonly children?: React.ReactNode })`
const nested = `React.createElement(View, { pad: '16px', id: 'parent' }, React.createElement(View, { pad: undefined, id: 'child' }))`

for (const mode of dynamicModes) {
  test.describe(`atomHoist=${mode.atomHoist ?? 'off'}, singleCss=${mode.singleCss}`, () => {
    test('undefined child padding stays initial when parent padding is dynamic', async ({
      page,
    }) => {
      // Given: different source sites, real public WASM output and React SSR.
      const html = renderDynamicSite(
        `export function View({ outer, inner }: { readonly outer: string; readonly inner: string | undefined }) { return <Box p={outer} data-testid="parent"><Box p={inner} data-testid="child" /></Box>; }`,
        `React.createElement(View, { outer: '16px', inner: undefined })`,
        mode,
      )
      // When: Chromium applies the generated CSS to the generated markup.
      await page.setContent(html)
      // Then: an omitted child variable cannot inherit the parent's value.
      expect(
        await page
          .getByTestId('parent')
          .evaluate((el) => getComputedStyle(el).padding),
      ).toBe('16px')
      expect(
        await page
          .getByTestId('child')
          .evaluate((el) => getComputedStyle(el).padding),
      ).toBe('0px')
    })

    test('recursive same-site padding does not inherit a missing value', async ({
      page,
    }) => {
      // Given: one compiled View rendered twice, nested at the same dynamic site.
      const html = renderDynamicSite(
        `export function View${parameters} { return <Box p={pad} id={id}>{children}</Box>; }`,
        nested,
        mode,
      )
      // When: Chromium resolves the omitted inner value.
      await page.setContent(html)
      // Then: the parent's present inline value wins; the child starts at zero.
      expect(
        await page
          .locator('#parent')
          .evaluate((el) => getComputedStyle(el).padding),
      ).toBe('16px')
      expect(
        await page
          .locator('#child')
          .evaluate((el) => getComputedStyle(el).padding),
      ).toBe('0px')
    })

    for (const consumer of [
      {
        name: 'before pseudo-element',
        props: `_before={{ content: '""', p: pad }}`,
        children: '',
        pseudo: '::before',
      },
      {
        name: 'direct descendant',
        props: `selectors={{ '& > span': { p: pad } }}`,
        children: '<span id="consumer" />',
        pseudo: null,
      },
    ] as const) {
      test(`valid dynamic padding reaches the ${consumer.name} consumer`, async ({
        page,
      }) => {
        // Given: the value belongs to the owner, not the consumer subject.
        const html = renderDynamicSite(
          `export function View${parameters} { return <Box ${consumer.props} id={id}>${consumer.children}</Box>; }`,
          `React.createElement(View, { pad: '16px', id: 'parent' })`,
          mode,
        )
        // When: Chromium inherits the generated variable into the consumer.
        await page.setContent(html)
        // Then: resetting on the pseudo-element or span would incorrectly erase it.
        expect(
          await page
            .locator(consumer.pseudo ? '#parent' : '#consumer')
            .evaluate(
              (el, pseudo) => getComputedStyle(el, pseudo).padding,
              consumer.pseudo,
            ),
        ).toBe('16px')
      })
    }

    test('undefined dynamic color retains normal computed parent inheritance', async ({
      page,
    }) => {
      // Given: a red dynamic value on an ancestor whose computed color is green.
      const html = renderDynamicSite(
        `export function View${parameters} { return <Box color={pad} id={id}>{children}</Box>; }`,
        `React.createElement(View, { pad: 'red', id: 'ancestor' }, React.createElement('section', { style: { color: 'green' } }, React.createElement(View, { pad: undefined, id: 'child' })))`,
        mode,
      )
      // When: Chromium resolves the missing child color.
      await page.setContent(html)
      // Then: color inherits the immediate parent's computed green, not stale red.
      expect(
        await page
          .locator('#child')
          .evaluate((el) => getComputedStyle(el).color),
      ).toBe('rgb(0, 128, 0)')
    })

    test('theme and manual variables remain inherited beside dynamic resets', async ({
      page,
    }) => {
      // Given: authored variables and a generated dynamic value share an owner.
      const html = renderDynamicSite(
        `export function View${parameters} { return <Box p={pad} color="$brand" bg="var(--manual)" id={id}>{children}</Box>; }`,
        `React.createElement('section', { style: { '--brand': 'green', '--manual': 'rgb(1, 2, 3)' } }, ${nested})`,
        mode,
      )
      // When: the child omits only the generated variable.
      await page.setContent(html)
      // Then: neither theme nor manual custom properties are reset.
      expect(
        await page
          .locator('#child')
          .evaluate((el) => [
            getComputedStyle(el).color,
            getComputedStyle(el).backgroundColor,
          ]),
      ).toEqual(['rgb(0, 128, 0)', 'rgb(1, 2, 3)'])
    })

    for (const condition of [
      { name: 'responsive', props: 'p={[null, pad]}', active: 'viewport' },
      { name: 'hover', props: '_hover={{ p: pad }}', active: 'hover' },
    ] as const) {
      test(`${condition.name} padding applies when active`, async ({
        page,
      }) => {
        // Given: dynamic padding available only under the selected condition.
        const html = renderDynamicSite(
          `export function View${parameters} { return <Box ${condition.props} id={id}>content</Box>; }`,
          `React.createElement(View, { pad: '16px', id: 'parent' })`,
          mode,
        )
        await page.setViewportSize({ width: 1600, height: 900 })
        await page.setContent(html)
        // When: the condition is active (desktop viewport or actual hover).
        if (condition.active === 'hover') await page.locator('#parent').hover()
        // Then: the inline value wins over the unconditional own reset.
        expect(
          await page
            .locator('#parent')
            .evaluate((el) => getComputedStyle(el).padding),
        ).toBe('16px')
      })

      test(`${condition.name} missing value omits its site class and reset outside the condition`, async ({
        page,
      }) => {
        // Given: same-site recursion while neither conditional consumer is active.
        const html = renderDynamicSite(
          `export function View${parameters} { return <Box ${condition.props} id={id}>{children}<Box id={id + '-omitted'} /></Box>; }`,
          nested,
          mode,
        )
        await page.setViewportSize({ width: 320, height: 900 })
        await page.mouse.move(319, 899)
        // When: Chromium computes variables before media/hover rules can apply.
        await page.setContent(html)
        // Then: the absent owner behaves like an actual Box with the prop omitted.
        expect(
          await page
            .locator('#child')
            .evaluate((el) => Array.from(el.classList)),
        ).toEqual([])
        expect(
          await page.locator('#child').evaluate((el) => Array.from(el.style)),
        ).toEqual([])
        expect(
          await page
            .locator('#child')
            .evaluate((el) => getComputedStyle(el).padding),
        ).toBe(
          await page
            .locator('#parent-omitted')
            .evaluate((el) => getComputedStyle(el).padding),
        )
        // Present owners retain exactly one unconditional OWN reset; absent ones inherit.
        const resets = await page.locator('#parent').evaluate((parent) => {
          const child = document.querySelector('#child')
          if (
            !(parent instanceof HTMLElement) ||
            !(child instanceof HTMLElement)
          )
            throw new TypeError('Expected rendered elements')
          return Array.from(parent.style)
            .filter((name) => name.startsWith('--'))
            .map((name) => ({
              present: getComputedStyle(parent).getPropertyValue(name).trim(),
              missing: getComputedStyle(child).getPropertyValue(name).trim(),
              ownRules: Array.from(document.styleSheets)
                .flatMap((sheet) => Array.from(sheet.cssRules))
                .filter(
                  (rule) =>
                    rule instanceof CSSStyleRule &&
                    rule.style.getPropertyValue(name).trim() === 'initial' &&
                    /^\.[\w-]+(?:\.[\w-]+)*$/.test(rule.selectorText) &&
                    parent.matches(rule.selectorText),
                ).length,
            }))
        })
        expect(resets.length).toBeGreaterThan(0)
        for (const reset of resets)
          expect(reset).toEqual({
            present: '16px',
            missing: '16px',
            ownRules: 1,
          })
      })
    }

    test('parent descendant padding crosses an absent same-site owner like static CSS', async ({
      page,
    }) => {
      // Given: the same compiled selector site is rendered with present and absent values.
      const html = renderDynamicSite(
        `export function View${parameters} { return <Box selectors={{ '& .child': { p: pad } }} id={id}>{children}<span className="child" id={id + '-consumer'} /><Box id={id + '-omitted'} /></Box>; }`,
        `React.createElement(React.Fragment, null, ${nested}, React.createElement('style', null, '.static-control .child { padding: 16px; }'), React.createElement('section', { className: 'static-control' }, React.createElement('section', { id: 'control-owner' }, React.createElement('span', { className: 'child', id: 'control-consumer' }))))`,
        mode,
      )
      // When: Chromium applies the ancestor selector through the absent nested owner.
      await page.setContent(html)
      // Then: no site class or local reset may block the inherited selector variable.
      const absent = await page.locator('#child').evaluate((el) => {
        if (!(el instanceof HTMLElement))
          throw new TypeError('Expected rendered element')
        const parent = document.querySelector('#parent')
        if (!(parent instanceof HTMLElement))
          throw new TypeError('Expected rendered parent')
        return {
          classes: Array.from(el.classList),
          inline: Array.from(el.style),
          inherited: Array.from(parent.style)
            .filter((name) => name.startsWith('--'))
            .map((name) => getComputedStyle(el).getPropertyValue(name).trim()),
          padding: getComputedStyle(el).padding,
        }
      })
      expect(absent.classes).toEqual([])
      expect(absent.inline).toEqual([])
      expect(absent.inherited.length).toBeGreaterThan(0)
      expect(absent.inherited.every((value) => value === '16px')).toBe(true)
      expect(absent.padding).toBe(
        await page
          .locator('#parent-omitted')
          .evaluate((el) => getComputedStyle(el).padding),
      )
      const control = await page
        .locator('#control-consumer')
        .evaluate((el) => getComputedStyle(el).padding)
      expect(control).toBe('16px')
      expect(
        await page
          .locator('#child-consumer')
          .evaluate((el) => getComputedStyle(el).padding),
      ).toBe(control)
    })
  })
}
