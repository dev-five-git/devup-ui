import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { runInNewContext } from 'node:vm'

const require = createRequire(import.meta.url)
const landing = createRequire(
  new URL('../../../apps/landing/package.json', import.meta.url),
)
const React = landing('react')
assert.ok(
  !process.argv[4] || process.env.VALUE_EVALUATION_QA === '1',
  'alternate WASM is private QA only',
)
const wasm = require(process.argv[4] ?? '../pkg/index.js')
const cases = Object.freeze({
  'extract_conditional_style_props-2': ['Box', 'margin', 'a === b ? c : d'],
  'extract_dynamic_logical_case-3': ['Box', 'margin', 'a ?? b'],
  'extract_responsive_conditional_style_props-3': [
    'Box',
    'margin',
    'a === b ? c : [d, e, f, "2px"]',
  ],
  'extract_responsive_conditional_style_props-4': [
    'Box',
    'margin',
    'a === b ? c : [d, e, f, x === y ? "4px" : "2px"]',
  ],
  'extract_responsive_conditional_style_props-5': [
    'Box',
    'margin',
    'a === b ? [d, e, f, x === y ? "4px" : "2px"] : c',
  ],
  member_expression_with_dynamic_values: [
    'Box',
    'bg',
    '({ a: first, b: second })[key]',
  ],
  props_direct_hybrid_responsive_select: [
    'Flex',
    'gap',
    '[[a, 1, c], [d, e, 2]][idx]',
  ],
  props_direct_variable_array_responsive_select: [
    'Flex',
    'gap',
    '[[a, b, c], [d, e, f]][idx]',
  ],
  'props_direct_variable_array_responsive_select-2': [
    'Flex',
    'gap',
    '[, [d, e, f]][idx]',
  ],
  props_direct_variable_object_responsive_select: [
    'Flex',
    'gap',
    '{ 0: [a, b, c], "1": [d, e, f] }[idx]',
  ],
  logical_or: ['Box', 'margin', 'a || b'],
  logical_and: ['Box', 'margin', 'a && b'],
  getter_member: ['Box', 'margin', 'store.value'],
  null_member: ['Box', 'margin', 'a === b ? c : store.value'],
  index_side_effects: ['Flex', 'gap', '[[a, b, c], [d, e, f]][index()]'],
})
for (const fixture of Object.values(cases)) Object.freeze(fixture)
const name = process.argv[2]
assert.ok(Object.hasOwn(cases, name), `unknown fixture ${name}`)
const [component, prop, expression] = cases[name]
const singleCss = process.argv[3] === 'single'
assert.ok(['single', 'ordinary'].includes(process.argv[3]))
wasm.resetBuildState()
wasm.setDebug(false)
wasm.registerTheme({ breakpoints: [0, 100, 200, 300, 400] })
const source = `import { ${component} } from '@devup-ui/react'; globalThis.element = <${component} ${prop}={${expression}} />;`
const output = wasm.codeExtract(
  'value.tsx',
  source,
  '@devup-ui/react',
  'df',
  singleCss,
  false,
  false,
  {},
)
const code = output.code
const css =
  wasm.getCss(null, false) + (singleCss ? '' : '\n' + (output.css ?? ''))
output.free()
// Transpile only JSX; execute both programs in native Node VM contexts below.
const programs = JSON.parse(
  execFileSync(
    'bun',
    [
      '--eval',
      `
const input = await Bun.stdin.text();
const transpiler = new Bun.Transpiler({loader:'tsx', tsconfig:{compilerOptions:{jsx:'react',jsxFactory:'React.createElement'}}});
process.stdout.write(JSON.stringify(JSON.parse(input).map(code => transpiler.transformSync(code.replace(/import\\s*(?:\\{[^}]*\\}\\s*from\\s*)?["'][^"']+["'];?/g,'')))));
`,
    ],
    {
      input: JSON.stringify([source, code]),
      encoding: 'utf8',
      timeout: 30_000,
      cwd: fileURLToPath(new URL('../../../', import.meta.url)),
    },
  ),
)

// Fixtures are immutable; each execution owns its trace and read counters.
const scenarios = Object.freeze([
  ...[true, false].flatMap((equal) =>
    [0, 1].map((index) => Object.freeze({ equal, index })),
  ),
  ...[0, false, '', null, undefined].map((left) =>
    Object.freeze({ equal: true, index: 0, left, nullish: true }),
  ),
  ...[0, false, ''].map((left) =>
    Object.freeze({ equal: true, index: 0, left, nullish: true, tdz: 'b' }),
  ),
  Object.freeze({ equal: true, index: 0, throwing: 'd,e,f,x,y' }),
  Object.freeze({ equal: false, index: 1, throwing: 'd,e,f,x,y' }),
  Object.freeze({ equal: false, index: 1, throwing: 'c' }),
  Object.freeze({ equal: true, index: 0, throwing: 'c' }),
  Object.freeze({ equal: true, index: 0, tdz: 'd' }),
  Object.freeze({ equal: false, index: 1, tdz: 'd' }),
  Object.freeze({ equal: false, index: 1, tdz: 'c' }),
  Object.freeze({ equal: true, index: 0, tdz: 'c' }),
  Object.freeze({
    equal: true,
    index: 0,
    throwing: 'b',
    left: '7px',
    nullish: true,
  }),
  Object.freeze({ equal: true, index: 0, throwing: 'second' }),
  Object.freeze({ equal: true, index: 0, changing: true }),
  Object.freeze({ equal: true, index: 0, changingControl: true }),
  Object.freeze({ equal: true, index: 0, nullMember: true }),
  Object.freeze({ equal: false, index: 1, nullMember: true }),
])
function evaluate(program, scenario) {
  const trace = []
  const reads = new Map()
  const values = {
    a: scenario.equal ? 'same' : 'other',
    b: 'same',
    c: '11px',
    d: '22px',
    e: '33px',
    f: '44px',
    x: scenario.index,
    y: 0,
    first: 'red',
    second: 'blue',
    idx: scenario.index,
    key: scenario.index ? 'b' : 'a',
  }
  if (prop === 'gap') Object.assign(values, { a: '11px', b: '12px', c: '13px' })
  if (scenario.nullish) Object.assign(values, { a: scenario.left, b: '55px' })
  const sandbox = { React, Box: component, Flex: component }
  const read = (key) => {
    trace.push(key)
    const count = reads.get(key) ?? 0
    reads.set(key, count + 1)
    if (scenario.throwing?.split(',').includes(key))
      throw new ReferenceError(key)
    if (scenario.changingControl && key === 'a') return count ? 'other' : 'same'
    if (scenario.changing && (key === 'idx' || key === 'key')) {
      return key === 'idx' ? count % 2 : count % 2 ? 'b' : 'a'
    }
    return values[key]
  }
  for (const key of Object.keys(values))
    Object.defineProperty(sandbox, key, { get: () => read(key) })
  Object.assign(sandbox, {
    store: scenario.nullMember
      ? null
      : {
          get value() {
            trace.push('getter')
            return '66px'
          },
        },
    index: () => {
      trace.push('index')
      return scenario.index
    },
  })
  try {
    runInNewContext(
      `(() => { ${program}\n${scenario.tdz ? `let ${scenario.tdz};` : ''} })()`,
      sandbox,
      { timeout: 1000 },
    )
    return { trace, element: sandbox.element, error: null }
  } catch (error) {
    // Cross-realm errors are serialized at this execution boundary, never ignored.
    assert.ok(error && typeof error.name === 'string')
    return { trace, element: null, error: error.name }
  }
}

function selectedValues(element) {
  const classes = new Set((element.props.className ?? '').split(/\s+/))
  const selected = new Map()
  const property = prop === 'bg' ? 'background' : prop
  // Parse nested rule blocks, carrying the actual media breakpoint into leaf rules.
  function rules(text, level) {
    let cursor = 0
    while (cursor < text.length) {
      const open = text.indexOf('{', cursor)
      if (open < 0) break
      const header = text.slice(cursor, open).trim()
      let end = open + 1
      let depth = 1
      while (depth && end < text.length) {
        if (text[end] === '{') depth++
        if (text[end] === '}') depth--
        end++
      }
      assert.equal(depth, 0, 'balanced emitted CSS')
      const body = text.slice(open + 1, end - 1)
      if (header.startsWith('@')) {
        const minimum = /min-width:\s*(\d+)px/.exec(header)
        rules(body, minimum ? Number(minimum[1]) / 100 : level)
      } else {
        const identifiers = [...header.matchAll(/\.([\w-]+)/g)].map(
          (match) => match[1],
        )
        if (identifiers.some((className) => classes.has(className))) {
          for (const declaration of body.split(';')) {
            const colon = declaration.indexOf(':')
            if (declaration.slice(0, colon).trim() !== property) continue
            const raw = declaration.slice(colon + 1).trim()
            const variable = /^var\((--[^,)]+)\)$/.exec(raw)
            const value = variable ? element.props.style?.[variable[1]] : raw
            selected.set(
              level,
              value == null || value === false || value === ''
                ? null
                : String(value) === '0px'
                  ? '0'
                  : String(value),
            )
          }
        }
      }
      cursor = end
    }
  }
  rules(css, 0)
  return [...selected]
    .filter(([, value]) => value !== null)
    .sort(([a], [b]) => a - b)
}

const failures = []
for (const [index, scenario] of scenarios.entries()) {
  // Given: the authored expression, with observable identifier/getter/index reads.
  const authored = evaluate(programs[0], scenario)
  // When: execute public-WASM output with exactly the same runtime bindings.
  const compiled = evaluate(programs[1], scenario)
  const value = authored.element?.props[prop]
  const expected = (Array.isArray(value) ? Array.from(value) : [value]).flatMap(
    (entry, level) =>
      entry == null || entry === false || entry === ''
        ? []
        : [
            [
              level,
              prop === 'gap' && typeof entry === 'number' && entry !== 0
                ? `${entry * 4}px`
                : String(entry) === '0px'
                  ? '0'
                  : String(entry),
            ],
          ],
  )
  const actual = compiled.element ? selectedValues(compiled.element) : []
  const evidence = {
    name,
    mode: process.argv[3],
    index,
    scenario,
    authored: {
      trace: authored.trace,
      error: authored.error,
      values: expected,
    },
    compiled: { trace: compiled.trace, error: compiled.error, values: actual },
  }
  try {
    // Then: selection, values, errors and exact source evaluation order/count agree.
    assert.deepEqual(compiled.trace, authored.trace)
    assert.equal(compiled.error, authored.error)
    assert.deepEqual(actual, expected)
  } catch (error) {
    if (!(error instanceof assert.AssertionError)) throw error
    failures.push(evidence)
  }
}
if (process.env.VALUE_EVALUATION_QA === '1') {
  process.stdout.write(
    JSON.stringify({
      name,
      mode: process.argv[3],
      scenarios: scenarios.length,
      failures,
    }) + '\n',
  )
} else {
  assert.deepEqual(failures, [], JSON.stringify(failures, null, 2))
}
