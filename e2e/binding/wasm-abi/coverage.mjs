/**
 * ABI coverage of the generated WASM package, reported apart from the Rust
 * line coverage: every wrapper tarpaulin skips (`cfg(not(tarpaulin_include))`)
 * is only reached through these JS calls.
 */

/** Parameters the boundary deliberately leaves unvalidated, with the reason. */
export const UNVALIDATED = {
  'hasDevupUI.import_aliases':
    'only read to find aliased imports; bindings/argument-validation.cjs never declared it',
}

/** Parse `export function name(a: T, b?: U): R;` lines of a wasm-bindgen `.d.ts`. */
export function parseDeclarations(dts) {
  const functions = {}
  for (const [, name, list] of dts.matchAll(
    /^export function (\w+)\(([^)]*)\)/gm,
  )) {
    functions[name] = list
      .split(',')
      .map((parameter) => parameter.trim())
      .filter(Boolean)
      .map((parameter) => parameter.replace(/\??:.*$/s, ''))
  }
  return functions
}

/**
 * Members of the generated `Output` class: its getters, `free` and
 * `Symbol.dispose`, named as the public scenarios label their steps.
 */
export function parseOutputMembers(dts) {
  const body = /^export class Output \{([\s\S]*?)^\}/m.exec(dts)?.[1] ?? ''
  return [
    ...body.matchAll(
      /^\s*(?:readonly (\w+):|(\w+)\(\)|\[(Symbol\.\w+)\]\(\))/gm,
    ),
  ].map(([, getter, method, symbol]) =>
    symbol ? `Symbol(${symbol})` : (getter ?? method),
  )
}

/**
 * Count how the public scenarios reached each export: as a recorded step, or
 * as one of the observers every step snapshots before and after it runs.
 */
export function countScenarioCalls(records) {
  const calls = {}
  const add = (name, count = 1) => (calls[name] = (calls[name] ?? 0) + count)
  for (const record of records) {
    if (record.kind !== 'step') continue
    add(record.label)
    for (const state of [record.before, record.after]) {
      for (const name of Object.keys(state.exports)) add(name)
      for (const name of Object.keys(state.config)) add(name)
      add('getThemeInterface')
      add('getCss', state.css.length)
    }
  }
  return calls
}

/**
 * Join the declarations, the scenario calls and the rejection cases into one
 * report. `ok` is false when an export is never called, or when an argument
 * position has no rejection case and is not listed in UNVALIDATED.
 */
export function evaluate({ declarations, members, calls, cases }) {
  const rejected = {}
  for (const test of cases) {
    const key = `${test.fn}.${test.parameter}`
    rejected[key] = (rejected[key] ?? 0) + 1
  }
  const problems = []
  const rows = Object.entries(declarations)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, parameters]) => {
      const called = calls[name] ?? 0
      if (called === 0) problems.push(`${name}: never called`)
      const positions = parameters.map((parameter, index) => {
        const key = `${name}.${parameter}`
        const count = rejected[key] ?? 0
        const position = cases.find(
          (test) => test.fn === name && test.parameter === parameter,
        )
        if (position && position.index !== index)
          problems.push(
            `${key}: declared at ${index}, validated at ${position.index}`,
          )
        if (count === 0 && !Object.hasOwn(UNVALIDATED, key))
          problems.push(`${key}: no rejection case`)
        return { parameter, rejected: count, exempt: count === 0 }
      })
      return { name, called, positions }
    })
  // The scenarios read every member on a live output and again after release.
  for (const member of members) {
    const called = ['live', 'release', 'released']
      .map((stage) => calls[`${stage}-${member}`] ?? 0)
      .reduce((sum, count) => sum + count, 0)
    if (called === 0) problems.push(`Output.${member}: never called`)
    rows.push({ name: `Output.${member}`, called, positions: [] })
  }
  for (const key of Object.keys(rejected)) {
    const [name, parameter] = key.split('.')
    if (!declarations[name]?.includes(parameter))
      problems.push(`${key}: rejection case for an undeclared parameter`)
  }
  for (const key of Object.keys(UNVALIDATED)) {
    const [name, parameter] = key.split('.')
    if (!declarations[name]?.includes(parameter) || rejected[key])
      problems.push(`${key}: stale UNVALIDATED entry`)
  }
  const positions = rows.flatMap((row) => row.positions)
  return {
    exports: rows.length,
    called: rows.filter((row) => row.called > 0).length,
    positions: positions.length,
    validated: positions.filter((position) => !position.exempt).length,
    cases: cases.length,
    rows,
    problems,
    ok: problems.length === 0,
  }
}

/** Render the report as Markdown for a CI step summary. */
export function report(result) {
  const lines = [
    '### WASM ABI coverage',
    '',
    `${result.called}/${result.exports} exports and \`Output\` members called, ${result.validated}/${result.positions} argument positions with rejection cases (${result.cases} cases).`,
    '',
    '| Export | Calls | Rejection cases per argument |',
    '| --- | ---: | --- |',
    ...result.rows.map(
      (row) =>
        `| \`${row.name}\` | ${row.called} | ${
          row.positions
            .map(
              (position) =>
                `${position.parameter}: ${position.exempt ? 'unvalidated' : position.rejected}`,
            )
            .join(', ') || '-'
        } |`,
    ),
    '',
  ]
  if (result.problems.length)
    lines.push(
      '**Problems**',
      '',
      ...result.problems.map((problem) => `- ${problem}`),
      '',
    )
  return lines.join('\n')
}
