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

type FixtureModule = {
  readonly path: string
  readonly code: string
}

function withWasmState<T>(action: () => T): T {
  const saved = {
    sheet: exportSheet(),
    classes: exportClassMap(),
    files: exportFileMap(),
    canonical: exportCanonicalMap(),
    debug: isDebug(),
    prefix: getPrefix(),
  }
  try {
    importSheet({})
    importClassMap({})
    importFileMap({})
    importCanonicalMap({})
    setDebug(false)
    setPrefix(null)
    return action()
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

function graphResolver(modules: readonly FixtureModule[]) {
  return (specifier: string, importer: string) => {
    const path = `${importer.slice(0, importer.lastIndexOf('/') + 1)}${specifier.slice(2)}.ts`
    return modules.find((module) => module.path === path)
  }
}

function artifacts(
  singleCss: boolean,
  parentSource: string | readonly FixtureModule[],
  childSource?: string,
) {
  return withWasmState(() => {
    const graph =
      typeof parentSource === 'string'
        ? [
            ...(childSource === undefined
              ? []
              : [{ path: '/src/child.ts', code: childSource }]),
            { path: '/src/parent.ts', code: parentSource },
          ]
        : parentSource
    setModuleResolver(graphResolver(graph))
    const compiled = graph.map((module) =>
      compile(module.path, module.code, singleCss),
    )
    const parent = compiled.at(-1)
    const child = compiled[0]
    if (parent === undefined || child === undefined) {
      throw new TypeError('Expected a nonempty composition graph')
    }
    expect([...parent.dependencies].sort()).toEqual(
      graph
        .slice(0, -1)
        .map((module) => module.path)
        .sort(),
    )
    return {
      css: singleCss
        ? getCss(undefined, false)
        : compiled.map((output) => output.css).join('\n'),
      parent: exportClasses(parent.code, 'base'),
      child: exportClasses(child.code, graph.length === 1 ? 'leaf' : 'base'),
      earlier: compiled
        .slice(0, -1)
        .map((output) => exportClasses(output.code, 'base')),
    }
  })
}

for (const singleCss of [true, false]) {
  test.describe(`css composition (${singleCss ? 'single' : 'per-file'} CSS)`, () => {
    for (const count of [3, 4]) {
      test(`terminal wins through ${count} imported modules without changing earlier exports`, async ({
        page,
      }) => {
        const graph = [
          {
            path: '/src/a.ts',
            code: `${api} export const base = css({ color: 'red', p: 2 });`,
          },
          {
            path: '/src/b.ts',
            code: `${api} import { base as child } from './a'; export const base = css(child, { color: 'orange', margin: 3 });`,
          },
          {
            path: '/src/c.ts',
            code: `${api} import { base as child } from './b'; export const base = css(child, { color: 'blue' });`,
          },
          {
            path: '/src/d.ts',
            code: `${api} import { base as child } from './c'; export const base = css(child, { color: 'purple', bg: 'yellow' });`,
          },
        ].slice(0, count)
        const emitted = artifacts(singleCss, graph)
        await page.setContent(`<style>${emitted.css}</style>
          <div id="parent" class="${emitted.parent}">Terminal</div>
          ${emitted.earlier.map((classes, index) => `<div id="earlier-${index}" class="${classes}">Earlier</div>`).join('')}`)
        await expect(page.locator('#parent')).toHaveCSS(
          'color',
          count === 3 ? 'rgb(0, 0, 255)' : 'rgb(128, 0, 128)',
        )
        await expect(page.locator('#parent')).toHaveCSS('padding', '8px')
        await expect(page.locator('#parent')).toHaveCSS('margin', '12px')
        if (count === 4) {
          await expect(page.locator('#parent')).toHaveCSS(
            'background-color',
            'rgb(255, 255, 0)',
          )
        }
        const colors = ['rgb(255, 0, 0)', 'rgb(255, 165, 0)', 'rgb(0, 0, 255)']
        for (const [index, color] of colors.slice(0, count - 1).entries()) {
          await expect(page.locator(`#earlier-${index}`)).toHaveCSS(
            'color',
            color,
          )
          await expect(page.locator(`#earlier-${index}`)).toHaveCSS(
            'padding',
            '8px',
          )
          if (index > 0) {
            await expect(page.locator(`#earlier-${index}`)).toHaveCSS(
              'margin',
              '12px',
            )
          }
        }
      })
    }

    test('mixed local aliases preserve responsive holes and hover through imported chains', async ({
      page,
    }) => {
      const emitted = artifacts(singleCss, [
        {
          path: '/src/a.ts',
          code: `${api} export const base = css(${leafRules});`,
        },
        {
          path: '/src/b.ts',
          code: `${api} import { base as first } from './a'; const alias = first; const local = css(alias, ${laterRules}); export const base = css(first, local);`,
        },
        {
          path: '/src/c.ts',
          code: `${api} import { base as child } from './b'; const alias = child; const local = css(alias, { color: ['cyan', null, null, null, 'gold'], _hover: { color: ['aqua', null, null, null, 'yellow'] } }); export const base = css(local, { bg: 'white' });`,
        },
      ])
      await page.setContent(`<style>${emitted.css}</style>
        <div id="parent" class="${emitted.parent}">Terminal</div>
        ${emitted.earlier.map((classes, index) => `<div id="earlier-${index}" class="${classes}">Earlier</div>`).join('')}`)
      const widths = [
        [
          320,
          'rgb(0, 255, 255)',
          'rgb(0, 255, 255)',
          'rgb(255, 165, 0)',
          'rgb(255, 99, 71)',
          'rgb(0, 0, 255)',
          'rgb(0, 0, 128)',
        ],
        [
          600,
          'rgb(255, 0, 0)',
          'rgb(255, 0, 0)',
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
          'rgb(0, 128, 0)',
          'rgb(0, 255, 0)',
        ],
        [
          1400,
          'rgb(255, 215, 0)',
          'rgb(255, 255, 0)',
          'rgb(128, 0, 128)',
          'rgb(128, 0, 128)',
          'rgb(255, 192, 203)',
          'rgb(0, 255, 0)',
        ],
      ] as const
      for (const [
        width,
        terminal,
        terminalHover,
        leaf,
        leafHover,
        middle,
        middleHover,
      ] of widths) {
        await page.setViewportSize({ width, height: 600 })
        await page.mouse.move(width - 1, 599)
        for (const [selector, color, hover] of [
          ['#parent', terminal, terminalHover],
          ['#earlier-0', leaf, leafHover],
          ['#earlier-1', middle, middleHover],
        ] as const) {
          await page.mouse.move(width - 1, 599)
          await expect(page.locator(selector)).toHaveCSS('color', color)
          await expect(page.locator(selector)).toHaveCSS('padding', '8px')
          await page.locator(selector).hover()
          await expect(page.locator(selector)).toHaveCSS('color', hover)
        }
        await expect(page.locator('#parent')).toHaveCSS('margin', '12px')
        await expect(page.locator('#earlier-1')).toHaveCSS('margin', '12px')
      }
    })

    const refusals = [
      {
        name: 'cycle',
        modules: [
          {
            path: '/src/a.ts',
            code: `${api}\nimport { base as child } from './b';\nexport const base = css(\nchild, { color: 'red' });`,
          },
          {
            path: '/src/b.ts',
            code: `${api}\nimport { base as child } from './a';\nexport const base = css(\nchild, { color: 'orange' });`,
          },
        ],
        error:
          /^\/src\/[ab]\.ts:4:1: `css\(\)` cannot use `child` at build time: .*cycl/,
      },
      {
        name: 'dynamic rule',
        modules: [
          {
            path: '/src/b.ts',
            code: `${api}\nexport const base = css(\n{ color: globalThis.tone });`,
          },
        ],
        error:
          /^\/src\/b\.ts:3:1: `css\(\)` cannot use `\{ color: globalThis\.tone \}` at build time: .*constant/,
      },
      {
        name: 'missing import',
        modules: [
          {
            path: '/src/a.ts',
            code: `${api}\nimport { base as child } from './missing';\nexport const base = css(\nchild, { color: 'red' });`,
          },
          {
            path: '/src/b.ts',
            code: `${api} import { base as child } from './a'; export const base = css(child, { color: 'orange' });`,
          },
        ],
        error:
          /^\/src\/a\.ts:4:1: `css\(\)` cannot use `child` at build time: .*module.*\.\/missing/,
      },
    ] as const
    for (const fixture of refusals) {
      test(`real WASM refuses an imported native ${fixture.name}`, () => {
        withWasmState(() => {
          setModuleResolver(graphResolver(fixture.modules))
          const source = `${api} import { base as child } from './b'; export const base = css(child, { color: 'blue' });`
          let failure: string | undefined
          try {
            compile('/src/terminal.ts', source, singleCss)
          } catch (error) {
            if (typeof error === 'string') {
              failure = error
            } else if (error instanceof Error) {
              failure = error.message
            } else {
              throw error
            }
          }
          expect(failure).toMatch(fixture.error)
        })
      })
    }

    for (const projection of [
      {
        name: 'failed member',
        source: `${api} import { seed, broken } from './a'; const holder = { broken }; export const base = css(seed, holder.broken, { color: 'blue' });`,
        refuses: true,
      },
      {
        name: 'direct failed import',
        source: `${api} import { seed, broken } from './a'; export const base = css(seed, broken, { color: 'blue' });`,
        refuses: true,
      },
      {
        name: 'shadowed failed import',
        source: `${api} import { seed, broken as alias } from './a'; export function compose() { const alias = 'external-card'; return css(seed, alias, { color: 'green' }); }`,
        refuses: false,
      },
      {
        name: 'unread failed sibling',
        source: `${api} import { seed, broken } from './a'; const holder = { broken, safe: 'external-card' }; export const base = css(seed, holder.safe, { color: 'blue' });`,
        refuses: false,
      },
    ] as const) {
      test(`real WASM projects consumption of a ${projection.name}`, () => {
        withWasmState(() => {
          setModuleResolver(
            graphResolver([
              {
                path: '/src/a.ts',
                code: `${api} export const seed = css({ color: 'red', p: 2 }); export const broken = css(seed, { color: globalThis.tone });`,
              },
            ]),
          )
          let failure: string | undefined
          let emitted: string | undefined
          try {
            emitted = compile(
              '/src/terminal.ts',
              projection.source,
              singleCss,
            ).code
          } catch (error) {
            if (typeof error === 'string') {
              failure = error
            } else if (error instanceof Error) {
              failure = error.message
            } else {
              throw error
            }
          }
          if (projection.refuses) {
            expect(failure).toMatch(
              /^\/src\/a\.ts:\d+:\d+: `css\(\)` cannot use/,
            )
            expect(failure).toContain('globalThis.tone')
          } else {
            expect(failure).toBeUndefined()
            expect(emitted).toContain('external-card')
          }
        })
      })
    }

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
