import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { runInNewContext } from 'node:vm'

import { Transpiler } from 'bun'

import {
  isolatedSuites,
  shorthands,
} from './dynamic-scaled-values-additional-fixtures.mjs'
import {
  files,
  fixtures,
  importedSource,
  modules,
  source,
} from './dynamic-scaled-values-fixtures.mjs'

const require = createRequire(import.meta.url)
const landing = createRequire(
  new URL('../../../apps/landing/package.json', import.meta.url),
)
const React = landing('react')
const { renderToStaticMarkup } = landing('react-dom/server')
const { jsx } = landing('react/jsx-runtime')
const { chromium, firefox, webkit } = require('@playwright/test')
const wasm = require('../pkg/index.js')
const transpiler = new Transpiler({
  loader: 'tsx',
  tsconfig: {
    compilerOptions: {
      jsx: 'react',
      jsxFactory: 'React.createElement',
    },
  },
})
function extract(single, order, suite) {
  wasm.resetBuildState()
  wasm.setDebug(false)
  wasm.setNamingRoot('/w42')
  wasm.seedFileMap(files)
  wasm.registerTheme({ breakpoints: [0, 600, 1000] })
  wasm.registerShorthands(shorthands)
  wasm.setModuleResolver((specifier) =>
    specifier === './types'
      ? { path: '/w42/types.ts', code: modules.get('/w42/types.ts') }
      : null,
  )
  const code = {}
  for (const index of order) {
    const output = wasm.codeExtract(
      files[index],
      index ? suite.importedSource : suite.source,
      '@devup-ui/react',
      'df',
      single,
      false,
      false,
      {},
    )
    code[files[index]] = output.code
    output.free()
  }
  const sheets = [
    wasm.getCss(null, false),
    ...files.map((_, index) => (single ? '' : wasm.getCss(index, false))),
  ]
  const result = {
    code: Object.fromEntries(files.map((file) => [file, code[file]])),
    sheets,
  }
  wasm.setModuleResolver(null)
  return result
}
let checks = 0
const suites = [
  {
    name: 'original-and-additional',
    fixtures: [...fixtures, { name: 'imported-number', props: ['width'] }],
    source,
    importedSource,
    baselineCss:
      '.anchor{padding:7px}.colorAnchor{color:rgb(33,44,55)}.opacityAnchor{opacity:0.37}',
  },
  ...isolatedSuites,
]
for (const [engine, launcher] of [
  ['chromium', chromium],
  ['firefox', firefox],
  ['webkit', webkit],
]) {
  const browser = await launcher.launch({ headless: true })
  try {
    for (const single of [false, true]) {
      for (const suite of suites) {
        const forward = extract(single, [0, 1], suite)
        assert.deepEqual(
          extract(single, [1, 0], suite),
          forward,
          'Full emitted output must not depend on extraction order',
        )
        const sandbox = {
          React,
          jsx,
          size: 100,
          pairs: [],
          readRight() {
            throw new Error('Falsy right branch executed')
          },
        }
        for (const code of Object.values(forward.code)) {
          runInNewContext(
            transpiler.transformSync(
              code.replace(
                /import\s*(?:\{[^}]*\}\s*from\s*)?["'][^"']+["'];?/g,
                '',
              ),
            ),
            sandbox,
            { timeout: 10000 },
          )
        }
        assert.equal(sandbox.pairs.length, suite.fixtures.length)
        const html = sandbox.pairs
          .map(
            ([dynamic, literal], index) =>
              `<section id="case${index}"><article>${renderToStaticMarkup(dynamic)}</article><article>${renderToStaticMarkup(literal)}</article></section>`,
          )
          .join('')
        const page = await browser.newPage()
        try {
          for (const width of [375, 800, 1440]) {
            await page.setViewportSize({ width, height: 900 })
            await page.setContent(
              `<!doctype html><style>${suite.baselineCss}article> *{display:block}${forward.sheets.join('\n')}</style>${html}`,
            )
            for (let index = 0; index < sandbox.pairs.length; index++) {
              const fixture = suite.fixtures[index]
              const selector =
                fixture.name === 'selector-child'
                  ? '> article > * > span'
                  : '> article > *'
              const targets = page.locator(`#case${index} ${selector}`)
              const values = []
              for (const side of [0, 1]) {
                if (fixture.hover) await targets.nth(side).hover()
                values.push(
                  await targets.nth(side).evaluate((element, props) => {
                    const style = getComputedStyle(element)
                    return props.map((prop) => style[prop])
                  }, fixture.props),
                )
              }
              assert.deepEqual(
                values[0],
                values[1],
                `${fixture.name}, single=${single}, width=${width}`,
              )
              if (fixture.expected)
                assert.deepEqual(
                  values[1],
                  fixture.expected,
                  `${fixture.name}: exact omitted-prop control`,
                )
              checks++
            }
          }
        } finally {
          await page.close()
        }
      }
    }
    process.stdout.write(
      JSON.stringify({
        checks,
        engine,
        version: browser.version(),
        fixtures: suites.reduce(
          (count, suite) => count + suite.fixtures.length,
          0,
        ),
        cssModes: 2,
        orders: 2,
        widths: 3,
      }) + '\n',
    )
  } finally {
    wasm.resetBuildState()
    await browser.close()
  }
}
