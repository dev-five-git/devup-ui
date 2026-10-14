import { expect, test } from '@playwright/test'

import {
  cascadeModes,
  cascadeOrders,
  compileCascadePair,
} from './environment-cascade-driver'

for (const mode of cascadeModes) {
  test.describe(`cascade pair: ${mode.name}`, () => {
    test('full output is identical when seeded extraction order reverses', async ({
      browserName,
    }, testInfo) => {
      // Given: D9 assigns both original IDs and CSS file numbers in path order.
      const forward = compileCascadePair(mode, cascadeOrders[0], true)
      // When: sources arrive BA rather than AB.
      const reverse = compileCascadePair(mode, cascadeOrders[1], true)
      // Then: compare complete source/code/markup/CSS/path bytes, without normalization.
      await testInfo.attach(`${browserName}-seeded-full-output`, {
        body: JSON.stringify({ forward, reverse }, null, 2),
        contentType: 'application/json',
      })
      expect(reverse).toEqual(forward)
    })

    for (const seeded of [false, true]) {
      for (const extraction of cascadeOrders) {
        for (const delivery of cascadeOrders) {
          for (const width of [320, 1440]) {
            test(`seeded=${seeded}, extract=${extraction.join('')}, sheets=${delivery.join('')}, width=${width}`, async ({
              page,
            }, testInfo) => {
              // Given: real public WASM compiles two unchanged authored files.
              // The unseeded case is essential: 1968's private D9 counters mask the alias.
              const pair = compileCascadePair(mode, extraction, seeded)
              await testInfo.attach('raw-public-output', {
                body: JSON.stringify(pair, null, 2),
                contentType: 'application/json',
              })
              const declarations = /(?:flex-direction|display):/
              if (mode.singleCss || mode.atomHoist !== undefined) {
                expect(pair.shared).toMatch(declarations)
                expect(pair.A.css).not.toMatch(declarations)
                expect(pair.B.css).not.toMatch(declarations)
              } else {
                expect(pair.shared).not.toMatch(declarations)
                expect(pair.A.css).toMatch(declarations)
                expect(pair.B.css).toMatch(declarations)
              }
              const sheets = [
                { id: 'shared', css: pair.shared },
                ...delivery.map((id) => ({ id, css: pair[id].css })),
              ]
              await page.route('https://cascade.test/*.css', async (route) => {
                const sheet = sheets.find(
                  ({ id }) =>
                    route.request().url() === `https://cascade.test/${id}.css`,
                )
                if (!sheet)
                  throw new TypeError('Unexpected cascade stylesheet request')
                await route.fulfill({
                  status: 200,
                  contentType: 'text/css',
                  body: sheet.css,
                })
              })
              await page.setViewportSize({ width, height: 900 })
              // When: Chromium loads independent stylesheet links in explicit AB or BA order.
              await page.setContent(
                `<!doctype html><html><head>${sheets.map(({ id }) => `<link rel="stylesheet" href="https://cascade.test/${id}.css">`).join('')}</head><body>${pair.A.markup}${pair.B.markup}</body></html>`,
                { waitUntil: 'load' },
              )
              // Then: #751 placement stays intact and no later base rule crosses file scopes.
              expect(
                await page.evaluate(() =>
                  Array.from(document.styleSheets, (sheet) => sheet.href),
                ),
              ).toEqual(
                sheets.map(({ id }) => `https://cascade.test/${id}.css`),
              )
              expect(
                await page.getByTestId('A').evaluate((element) => {
                  const style = getComputedStyle(element)
                  return [style.flexDirection, style.display]
                }),
              ).toEqual(width === 1440 ? ['row', 'flex'] : ['column', 'none'])
              expect(
                await page.getByTestId('B').evaluate((element) => {
                  const style = getComputedStyle(element)
                  return [style.flexDirection, style.display]
                }),
              ).toEqual(['column', 'none'])
            })
          }
        }
      }
    }
  })
}
