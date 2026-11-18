import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterAll, describe, expect, it } from 'bun:test'

// Devup UI against the real StyleX compiler: each case is compiled by
// `@stylexjs/babel-plugin` and by the WASM build, and the rules must agree.
const wasm = await import('../../../../bindings/devup-ui-wasm/pkg/index.js')

interface BabelResult {
  code: string
  metadata: { stylex: unknown }
}
interface Babel {
  transformSync(code: string, options: object): BabelResult
}
interface StylexPlugin {
  processStylexRules(rules: unknown, options: object): string
}

const require = createRequire(import.meta.url)
const pluginPath = require.resolve('@stylexjs/babel-plugin')
const stylexPlugin: StylexPlugin = require(pluginPath)
// The compiler the plugin itself runs on, as the plugin's own dependency
const babel: Babel = createRequire(pluginPath)('@babel/core')

const root = mkdtempSync(join(tmpdir(), 'devup-ui-stylex-when-'))
mkdirSync(join(root, 'src'))
afterAll(() => {
  wasm.setModuleResolver(null)
  rmSync(root, { recursive: true, force: true })
})

const IMPORT = "import * as stylex from '@stylexjs/stylex';\n"
const MARKERS = `${IMPORT}export const card = stylex.defineMarker();\nexport const row = stylex.defineMarker();\n`
writeFileSync(join(root, 'src', 'markers.stylex.js'), MARKERS)

function upstream(file: string, code: string) {
  const result = babel.transformSync(code, {
    filename: file,
    babelrc: false,
    configFile: false,
    parserOpts: { plugins: ['jsx'] },
    plugins: [
      [
        pluginPath,
        {
          dev: false,
          runtimeInjection: false,
          unstable_moduleResolution: { type: 'commonJS', rootDir: root },
        },
      ],
    ],
  })
  return {
    code: result.code,
    css: stylexPlugin.processStylexRules(result.metadata.stylex, {
      useLayers: false,
    }),
  }
}

// The one module the cases import, as the build resolves it for Devup UI
function resolveModule(specifier: string) {
  return specifier === './markers.stylex.js'
    ? { path: join(root, 'src', 'markers.stylex.js'), code: MARKERS }
    : null
}

function devup(file: string, code: string) {
  wasm.setDebug(false)
  wasm.setLicenseBanner(false)
  wasm.setModuleResolver(resolveModule)
  const output = wasm.codeExtract(
    file,
    code,
    '@devup-ui/react',
    'df',
    true,
    false,
    false,
    {},
  )
  return { code: output.code, css: wasm.getCss(null, false) }
}

interface Rule {
  atRules: string[]
  selector: string
  declaration: string
}

/** The rules of flat CSS, with the at-rules each is nested in */
function parse(css: string, atRules: string[] = []): Rule[] {
  const rules: Rule[] = []
  let index = 0
  while (index < css.length) {
    const open = css.indexOf('{', index)
    if (open < 0) break
    const prelude = css.slice(index, open).trim()
    let depth = 1
    let close = open + 1
    while (depth > 0) {
      if (css[close] === '{') depth++
      if (css[close] === '}') depth--
      close++
    }
    const body = css.slice(open + 1, close - 1)
    if (prelude.startsWith('@')) {
      rules.push(...parse(body, [...atRules, prelude.replace(/\s+/g, '')]))
    } else {
      rules.push({ atRules, selector: prelude, declaration: body })
    }
    index = close
  }
  return rules
}

/**
 * A rule as a comparable text: its class as `C`, written as many times as it
 * is outside the at-rules, and markers by the name they are exported as.
 * StyleX also repeats the class once per enclosing at-rule; Devup UI orders
 * at-rules after the rules they refine instead, which those repeats are not
 * part of. Without cascade layers StyleX also raises longhands over
 * shorthands with `:not(#\#)`, where Devup UI orders them instead.
 */
function normalize(
  rule: Rule,
  markers: Record<string, string>,
  upstream: boolean,
) {
  const [, className] = /^\.([\w-]+)/.exec(rule.selector) ?? []
  let rest = rule.selector.replaceAll(':not(#\\#)', '')
  let repeats = 0
  while (rest.startsWith(`.${className}`)) {
    rest = rest.slice(className.length + 1)
    repeats++
  }
  if (upstream) repeats -= rule.atRules.length
  for (const [marker, name] of Object.entries(markers)) {
    rest = rest.replaceAll(`.${marker}`, `.<${name}>`)
  }
  return `${rule.atRules.join(' ')}|${'.C'.repeat(repeats)}${rest}{${rule.declaration}}`
}

/** The class each exported marker gives, by what the compiled code binds it to */
function markerClasses(code: string, upstream: boolean) {
  const pattern = upstream
    ? /export const (\w+) = \{\s*"?([\w-]+)"?: "[\w-]+",\s*\$\$css: true\s*\}/g
    : /export const (\w+) = "([\w-]+)"/g
  return Object.fromEntries(
    [...code.matchAll(pattern)].map(([, name, marker]) => [marker, name]),
  )
}

/** The rules each compiler gives, in order, for every property they set */
function byProperty(rules: string[]) {
  const properties: Record<string, string[]> = {}
  for (const rule of rules) {
    const property = /\{([\w-]+):/.exec(rule)?.[1] ?? ''
    ;(properties[property] ??= []).push(rule)
  }
  return properties
}

const CASES: Record<string, string> = {
  relations: `${IMPORT}const s = stylex.create({ a: { color: { default: 'black', ':hover': 'gray', ':focus': 'silver', [stylex.when.ancestor(':hover')]: 'red', [stylex.when.descendant(':focus')]: 'blue', [stylex.when.anySibling(':active')]: 'green', [stylex.when.siblingBefore(':hover')]: 'pink', [stylex.when.siblingAfter(':focus-visible')]: 'teal', [stylex.when.ancestor('[data-open]')]: 'olive' } } });\nexport const p = stylex.props(stylex.defaultMarker(), s.a);\n`,
  nested: `${IMPORT}const s = stylex.create({ a: { [stylex.when.ancestor(':hover')]: { color: 'red' }, backgroundColor: { default: 'white', '@media (min-width: 600px)': { default: 'gray', [stylex.when.descendant(':focus')]: 'yellow' } }, borderColor: { default: 'black', [stylex.when.ancestor(':hover')]: { default: 'red', ':focus': 'blue' } } } });\nexport const p = stylex.props(s.a);\n`,
  'named imports': `import { create, props, when, defaultMarker, defineMarker } from '@stylexjs/stylex';\nexport const local = defineMarker();\nconst s = create({ a: { color: { default: 'black', [when.ancestor(':hover', local)]: 'red', [when.siblingBefore(':active', defaultMarker())]: 'blue', [when.anySibling(':hover', 'my-marker')]: 'green' } } });\nexport const p = props(local, s.a);\n`,
  'imported markers': `${IMPORT}import { card, row } from './markers.stylex.js';\nconst s = stylex.create({ a: { color: { default: 'black', [stylex.when.ancestor(':hover', card)]: 'red', [stylex.when.siblingAfter(':focus', row)]: 'blue' } } });\nexport const p = stylex.props(card, row, s.a);\n`,
}

describe('stylex.when and markers against @stylexjs/babel-plugin', () => {
  const markerFile = join(root, 'src', 'markers.stylex.js')
  const upstreamMarkers = markerClasses(
    upstream(markerFile, MARKERS).code,
    true,
  )
  // The classes the module defining the markers writes, once the case is
  // compiled so that both read the same class names
  const devupMarkers = () =>
    markerClasses(devup(markerFile, MARKERS).code, false)

  it.each(Object.entries(CASES))('%s', (name, code) => {
    // StyleX hashes `defineMarker()` only in `.stylex` files
    const extension = code.includes('defineMarker') ? '.stylex.jsx' : '.jsx'
    const file = join(root, 'src', `${name.replace(' ', '-')}${extension}`)
    const expected = upstream(file, code)
    wasm.resetStateForTesting()
    const actual = devup(file, code)
    const imported = devupMarkers()
    const rules = (
      compiled: { code: string; css: string },
      isUpstream: boolean,
    ) => {
      const markers = {
        ...(isUpstream ? upstreamMarkers : imported),
        ...markerClasses(compiled.code, isUpstream),
      }
      return parse(compiled.css).map((rule) =>
        normalize(rule, markers, isUpstream),
      )
    }
    const upstreamRules = rules(expected, true)
    const devupRules = rules(actual, false)

    expect(devupRules.toSorted()).toEqual(upstreamRules.toSorted())
    expect(byProperty(devupRules)).toEqual(byProperty(upstreamRules))
    expect(upstreamRules.some((rule) => rule.includes(':where('))).toBe(true)
  })

  it('names imported markers as the module defining them does', () => {
    wasm.resetStateForTesting()
    expect(Object.values(devupMarkers())).toEqual(['card', 'row'])
    expect(Object.values(upstreamMarkers)).toEqual(['card', 'row'])
  })
})
