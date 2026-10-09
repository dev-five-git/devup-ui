import { expect, test } from '@playwright/test'

import {
  codeExtract,
  exportCanonicalMap,
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
  getPrefix,
  importCanonicalMap,
  importClassMap,
  importFileMap,
  importSheet,
  isDebug,
  setDebug,
  setModuleResolver,
  setPrefix,
} from '../bindings/devup-ui-wasm/pkg/index.js'

const api = "import { css } from '@devup-ui/react';"
const leafRules = `{
  color: ['orange', 'red', 'purple'], p: 2,
  _hover: { color: ['tomato', 'red', 'purple'] }
}`
const laterRules = `{
  color: ['blue', null, 'green', null, 'pink'], margin: 3,
  _hover: { color: ['navy', null, 'lime'] }
}`

function exportClasses(code: string, name: string): string {
  const literal = code.match(
    new RegExp(`export const ${name}\\s*=\\s*("(?:[^"\\\\]|\\\\.)*")\\s*;`),
  )?.[1]
  if (literal === undefined) {
    throw new TypeError(`Expected a compiled literal for ${name}: ${code}`)
  }
  const classes: unknown = JSON.parse(literal)
  if (typeof classes !== 'string') {
    throw new TypeError(`Expected class string for ${name}`)
  }
  expect(classes).toMatch(/^[a-zA-Z0-9_-]+(?: [a-zA-Z0-9_-]+)*$/)
  return classes
}

function compile(filename: string, source: string, singleCss: boolean) {
  const output = codeExtract(
    filename,
    source,
    '@devup-ui/react',
    'df',
    singleCss,
    false,
    false,
    {},
  )
  try {
    const css = singleCss ? '' : output.css
    if (css === undefined) {
      throw new TypeError(`Expected per-file CSS for ${filename}`)
    }
    return { code: output.code, css, dependencies: output.dependencies }
  } finally {
    output.free()
  }
}

function artifacts(
  singleCss: boolean,
  parentSource: string,
  childSource?: string,
) {
  const saved = {
    sheet: exportSheet(),
    classes: exportClassMap(),
    files: exportFileMap(),
    canonical: exportCanonicalMap(),
    debug: isDebug(),
    prefix: getPrefix(),
  }
  try {
    importClassMap({})
    importFileMap({})
    importCanonicalMap({})
    setDebug(false)
    setPrefix(null)
    setModuleResolver((specifier: string, importer: string) =>
      childSource !== undefined &&
      specifier === './child' &&
      importer === '/src/parent.ts'
        ? { path: '/src/child.ts', code: childSource }
        : undefined,
    )
    const child =
      childSource === undefined
        ? undefined
        : compile('/src/child.ts', childSource, singleCss)
    const parent = compile('/src/parent.ts', parentSource, singleCss)
    if (child !== undefined) {
      expect(parent.dependencies).toContain('/src/child.ts')
    }
    return {
      css: singleCss
        ? getCss(undefined, false)
        : `${child?.css ?? ''}\n${parent.css}`,
      parent: exportClasses(parent.code, 'base'),
      child: exportClasses(
        child?.code ?? parent.code,
        child === undefined ? 'leaf' : 'base',
      ),
    }
  } finally {
    setModuleResolver(null)
    importSheet(JSON.parse(saved.sheet))
    importClassMap(JSON.parse(saved.classes))
    importFileMap(JSON.parse(saved.files))
    importCanonicalMap(JSON.parse(saved.canonical))
    setDebug(saved.debug)
    setPrefix(saved.prefix)
  }
}

for (const singleCss of [true, false]) {
  test.describe(`css composition (${singleCss ? 'single' : 'per-file'} CSS)`, () => {
    test('later argument wins across the imported W35i leaf', async ({
      page,
    }) => {
      const emitted = artifacts(
        singleCss,
        `${api} import { base as child } from './child'; export const base = css(child, { color: 'blue', margin: 3 });`,
        `${api} export const base = css({ color: 'orange', p: 2 });`,
      )
      await page.setContent(`<style>${emitted.css}</style>
        <div id="parent" class="${emitted.parent}">Parent</div>
        <div id="child" class="${emitted.child}">Child</div>`)
      await expect(page.locator('#parent')).toHaveCSS('color', 'rgb(0, 0, 255)')
      await expect(page.locator('#parent')).toHaveCSS('padding', '8px')
      await expect(page.locator('#parent')).toHaveCSS('margin', '12px')
      await expect(page.locator('#child')).toHaveCSS(
        'color',
        'rgb(255, 165, 0)',
      )
      await expect(page.locator('#child')).toHaveCSS('padding', '8px')
    })

    for (const chain of [false, true]) {
      test(`later hover/responsive arguments win across ${chain ? 'a same-module three-level chain' : 'an imported leaf'}`, async ({
        page,
      }) => {
        const emitted = chain
          ? artifacts(
              singleCss,
              `${api}
              export const leaf = css(${leafRules});
              const middle = css(leaf, ${laterRules});
              export const base = css(middle, {
                color: ['cyan', null, null, null, 'gold'],
                _hover: { color: ['aqua', null, null, null, 'yellow'] }
              });`,
            )
          : artifacts(
              singleCss,
              `${api} import { base as child } from './child'; export const base = css(child, ${laterRules});`,
              `${api} export const base = css(${leafRules});`,
            )
        await page.setContent(`<style>${emitted.css}</style>
          <div id="parent" class="${emitted.parent}">Parent</div>
          <div id="child" class="${emitted.child}">Child</div>`)
        const widths = [
          [
            320,
            chain ? 'rgb(0, 255, 255)' : 'rgb(0, 0, 255)',
            chain ? 'rgb(0, 255, 255)' : 'rgb(0, 0, 128)',
            'rgb(255, 165, 0)',
            'rgb(255, 99, 71)',
          ],
          [
            600,
            'rgb(255, 0, 0)',
            'rgb(255, 0, 0)',
            'rgb(255, 0, 0)',
            'rgb(255, 0, 0)',
          ],
          [
            900,
            'rgb(0, 128, 0)',
            'rgb(0, 255, 0)',
            'rgb(128, 0, 128)',
            'rgb(128, 0, 128)',
          ],
          [
            1400,
            chain ? 'rgb(255, 215, 0)' : 'rgb(255, 192, 203)',
            chain ? 'rgb(255, 255, 0)' : 'rgb(0, 255, 0)',
            'rgb(128, 0, 128)',
            'rgb(128, 0, 128)',
          ],
        ] as const
        for (const [
          width,
          parentColor,
          hoverColor,
          childColor,
          childHoverColor,
        ] of widths) {
          await page.setViewportSize({ width, height: 600 })
          await page.mouse.move(width - 1, 599)
          await expect(page.locator('#parent')).toHaveCSS('color', parentColor)
          await expect(page.locator('#parent')).toHaveCSS('padding', '8px')
          await expect(page.locator('#parent')).toHaveCSS('margin', '12px')
          await expect(page.locator('#child')).toHaveCSS('color', childColor)
          await page.locator('#parent').hover()
          await expect(page.locator('#parent')).toHaveCSS('color', hoverColor)
          await page.locator('#child').hover()
          await expect(page.locator('#child')).toHaveCSS(
            'color',
            childHoverColor,
          )
        }
      })
    }
  })
}
