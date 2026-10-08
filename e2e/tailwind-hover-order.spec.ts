import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'

import { expect, test } from '@playwright/test'

import {
  codeExtractWithoutSourceMap,
  getCss,
  importClassMap,
  importFileMap,
  importSheet,
  registerTheme,
} from '../bindings/devup-ui-wasm/pkg/index.js'

const tailwindRequire = createRequire(
  new URL('../benchmark/next-tailwind/package.json', import.meta.url),
)
const {
  compile,
}: typeof import('../benchmark/next-tailwind/node_modules/tailwindcss/dist/lib.mjs') =
  tailwindRequire('tailwindcss')
const tailwindTheme = readFileSync(
  tailwindRequire.resolve('tailwindcss/theme.css'),
  'utf8',
)
const cases = [
  {
    classes: 'hover:bg-red-500 active:bg-blue-500',
    winners: ['blue', 'blue'],
  },
  {
    classes: 'md:hover:bg-red-500 active:bg-blue-500',
    winners: ['blue', 'red'],
  },
  {
    classes: 'hover:bg-red-500 focus:bg-green-500 active:bg-blue-500',
    winners: ['blue', 'blue'],
  },
  {
    classes: 'hover:bg-red-500 disabled:bg-gray-500',
    winners: ['gray', 'gray'],
  },
  {
    classes: 'dark:hover:bg-red-500 active:bg-blue-500',
    winners: ['red', 'red'],
  },
  {
    classes: 'hover:bg-red-500 active:bg-blue-500',
    props:
      '_media={{"(hover: hover)": {_hover: {bgColor:"#EF4444"}}}} _active={{bgColor:"#3B82F6"}}',
    winners: ['blue', 'blue'],
  },
] as const
const colors = {
  red: ['oklch(0.637 0.237 25.331)', 'rgb(239, 68, 68)'],
  blue: ['oklch(0.623 0.214 259.815)', 'rgb(59, 130, 246)'],
  green: ['oklch(0.723 0.219 149.579)', 'rgb(34, 197, 94)'],
  gray: ['oklch(0.551 0.027 264.364)', 'rgb(107, 114, 128)'],
}

for (const singleCss of [true, false]) {
  for (const fixture of [
    {
      props:
        '_hover={{bg:"red"}} _focus={{bg:"green"}} _active={{bg:"blue"}} _disabled={{bg:"gray"}}',
      css: '.a:hover{background:red}.b:focus{background:green}.c:active{background:blue}.d:disabled{background:gray}',
    },
    {
      props:
        'bg={["white","black"]} _hover={{bg:["red",null,"orange"]}} _focus={{bg:"green"}} _active={{bg:"blue"}} _disabled={{bg:"gray"}}',
      css: '.a{background:white}@media(min-width:480px){.b{background:black}}.c:hover{background:red}@media(min-width:768px){.d:hover{background:orange}}.e:focus{background:green}.f:active{background:blue}.g:disabled{background:gray}',
    },
  ]) {
    test(`Devup selector props unchanged, ${fixture.props}, singleCss=${singleCss}`, () => {
      importSheet({})
      importClassMap({})
      importFileMap({})
      registerTheme({})
      const output = codeExtractWithoutSourceMap(
        'selector-props.tsx',
        `import { Box } from '@devup-ui/react';\n<Box ${fixture.props} />`,
        '@devup-ui/react',
        'df',
        singleCss,
        false,
        false,
        {},
      )
      try {
        const actual = singleCss ? getCss(null, false) : output.css
        const expected = singleCss
          ? fixture.css
          : fixture.css.replace(/\.([a-g])/g, '.a-$1')
        expect(actual?.replace(/^\/\*[\s\S]*?\*\//, '')).toBe(expected)
      } finally {
        output.free()
      }
    })
  }
}

test.describe('Hover capability guard', () => {
  test.use({ hasTouch: true, isMobile: true })
  for (const singleCss of [true, false]) {
    test(`touch input keeps the base background, singleCss=${singleCss}`, async ({
      page,
    }) => {
      const classes = 'bg-white hover:bg-red-500'
      const native = await compile(`${tailwindTheme}\n@tailwind utilities;`)
      const oracleCss = native.build(classes.split(' '))
      importSheet({})
      registerTheme({})
      const output = codeExtractWithoutSourceMap(
        'hover-capability.tsx',
        `import { Box } from '@devup-ui/react';\n<Box className="${classes}" />`,
        '@devup-ui/react',
        'df',
        singleCss,
        false,
        false,
        {},
      )
      try {
        const matched = output.code.match(/className="([^"]+)"/)
        expect(matched).not.toBeNull()
        const devupCss = singleCss
          ? getCss(null, false)
          : getCss(null, false) + output.css
        expect(devupCss).toContain('(hover:hover)')
        for (const [className, css] of [
          [classes, oracleCss],
          [matched?.[1], devupCss],
        ]) {
          await page.setContent(
            `<!doctype html><style>${css}</style><div id="target" class="${className}" style="width:160px;height:80px">tap</div>`,
          )
          expect(
            await page.evaluate(() => matchMedia('(hover:hover)').matches),
          ).toBe(false)
          await page.locator('#target').tap()
          await expect(page.locator('#target')).toHaveCSS(
            'background-color',
            'rgb(255, 255, 255)',
          )
        }
      } finally {
        output.free()
      }
    })
  }
})

for (const singleCss of [true, false]) {
  for (const fixture of cases) {
    for (const [index, width] of [640, 1024].entries()) {
      const label = 'props' in fixture ? 'explicit Devup guard: ' : ''
      test(`${label}${fixture.classes}, ${width}px, singleCss=${singleCss}`, async ({
        page,
      }, testInfo) => {
        const disabled = fixture.classes.includes('disabled:')
        const native = await compile(`${tailwindTheme}\n@tailwind utilities;`)
        const oracleCss = native.build(fixture.classes.split(' '))
        importSheet({})
        registerTheme({})
        const props =
          'props' in fixture
            ? fixture.props
            : `className="${fixture.classes}" ${disabled ? 'disabled' : ''}`
        const source = `import { Box } from '@devup-ui/react';\n<Box as="button" ${props} />`
        const output = codeExtractWithoutSourceMap(
          'hover-order.tsx',
          source,
          '@devup-ui/react',
          'df',
          singleCss,
          false,
          false,
          {},
        )
        try {
          const matched = output.code.match(/className="([^"]+)"/)
          expect(matched).not.toBeNull()
          if (!singleCss) expect(output.cssFile).toBeDefined()
          const devupCss = singleCss
            ? getCss(null, false)
            : getCss(null, false) + output.css
          expect(devupCss).toContain('(hover:hover)')
          const expected = colors[fixture.winners[index]]
          await page.setViewportSize({ width, height: 400 })
          await page.emulateMedia({ colorScheme: 'dark' })
          const observed = []
          for (const [name, classes, css, color] of [
            ['Tailwind', fixture.classes, oracleCss, expected[0]],
            ['Devup', matched?.[1], devupCss, expected[1]],
          ]) {
            await page.setContent(
              `<!doctype html><html data-theme="dark"><head><style>${css}</style></head><body><button id="target" class="${classes}" ${disabled ? 'disabled' : ''} style="width:160px;height:80px">probe</button></body></html>`,
            )
            await page.mouse.move(width - 1, 399)
            const target = page.locator('#target')
            const baseline = await target.evaluate(
              (element) => getComputedStyle(element).backgroundColor,
            )
            await target.hover()
            if (fixture.classes.startsWith('md:') && width < 768) {
              await expect(target).toHaveCSS('background-color', baseline)
            } else if (!disabled) {
              await expect(target).toHaveCSS(
                'background-color',
                colors.red[name === 'Tailwind' ? 0 : 1],
              )
            }
            if (fixture.classes.includes('focus:')) {
              await target.focus()
              await expect(target).toHaveCSS(
                'background-color',
                colors.green[name === 'Tailwind' ? 0 : 1],
              )
            }
            await page.mouse.down()
            try {
              const held = await target.evaluate((element) => ({
                color: getComputedStyle(element).backgroundColor,
                hover: element.matches(':hover'),
                active: element.matches(':active'),
                focus: element.matches(':focus'),
                disabled: element.matches(':disabled'),
                hoverMedia: matchMedia('(hover:hover)').matches,
              }))
              expect(held.hover, name).toBe(true)
              expect(held.active, name).toBe(true)
              expect(held.hoverMedia, name).toBe(true)
              expect(held.disabled, name).toBe(disabled)
              expect(held.focus, name).toBe(!disabled)
              observed.push({ name, held })
              expect(held.color, name).toBe(color)
              if (name === 'Devup') {
                const screenshot = testInfo.outputPath('held.png')
                await page.screenshot({ path: screenshot })
                await testInfo.attach('held-Devup', {
                  path: screenshot,
                  contentType: 'image/png',
                })
              }
            } finally {
              await page.mouse.up()
            }
          }
          await testInfo.attach('cascade-evidence', {
            body: JSON.stringify({ source, oracleCss, devupCss, observed }),
            contentType: 'application/json',
          })
        } finally {
          output.free()
        }
      })
    }
  }
}
