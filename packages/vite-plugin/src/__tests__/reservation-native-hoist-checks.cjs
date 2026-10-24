const assert = require('node:assert/strict')
const { readFileSync, readdirSync } = require('node:fs')
const { resolve } = require('node:path')
const wasm = require('@devup-ui/wasm')

/** @param {string} css */
function declarations(css) {
  return [...css.matchAll(/\.([\w-]+)\{([^{}]+)\}/g)].map((match) => ({
    name: match[1],
    value: match[2],
  }))
}

/** @param {string} code */
function classes(code) {
  return [
    ...code.matchAll(/className\s*=\s*(?:\{\s*\[\s*)?["']([^"']+)["']/g),
  ].flatMap((match) => (match[1] ?? '').split(/\s+/))
}

/** @param {{root:string,mode:string,single:string,http:Record<string,string>,telemetry:ReturnType<typeof import('./reservation-native-observer.cjs').observe>,lazy?:boolean}} context */
function checkHoist({ root, mode, single, http, telemetry, lazy }) {
  const snapshot = telemetry.snapshot()
  const base = declarations(snapshot.css)
  const shared = base.filter((row) =>
    /color:orchid|padding:28px/.test(row.value ?? ''),
  )
  assert.equal(
    shared.length,
    2,
    'real shared supplier declarations exist once in base',
  )
  assert(
    shared.every((row) => /^[a-z0-9]+$/.test(row.name ?? '')),
    'shared global short names',
  )
  if (single !== 'single')
    assert(
      snapshot.extractions.some((row) => row.updatedBaseStyle),
      'live shared dirtiness',
    )
  const disk = readFileSync(resolve(root, 'df/devup-ui/devup-ui.css'), 'utf8')
  for (const row of shared)
    assert(disk.includes(`.${row.name}{`), 'live base publication')
  for (const [name, opacity] of [
    ['A', '.25'],
    ['B', '.75'],
  ]) {
    const output = snapshot.extractions.find((row) =>
      row.id.endsWith(`/physical/${name}.tsx`),
    )
    assert(output)
    assert(output.map)
    const code = output.code
    const ids = [...code.matchAll(/devup-ui-(\d+)\.css/g)].map((match) =>
      Number(match[1]),
    )
    const local = ids.map((id) => wasm.getCss(id, false)).join('\n')
    const rules = declarations(single === 'single' ? snapshot.css : local)
    const privateRule = rules.find((row) =>
      row.value?.includes(`opacity:${opacity}`),
    )
    assert(privateRule)
    assert(
      single === 'single' ||
        /^[a-z0-9]+-[a-z0-9]+$/.test(privateRule.name ?? ''),
      'private local short name',
    )
    assert(
      classes(code).includes(privateRule.name ?? ''),
      'private generated reference resolves',
    )
    if (single !== 'single')
      assert(
        !/color:orchid|padding:28px/.test(local),
        'no shared duplication locally',
      )
    const visible = declarations(snapshot.css + '\n' + local)
    for (const name of classes(code))
      assert.equal(
        visible.filter((row) => row.name === name).length,
        1,
        'every private class resolves once',
      )
    for (const match of local.matchAll(/var\((--[\w-]+)\)/g))
      assert(
        code.includes(`"${match[1]}"`),
        'local variable reference has an emitted definition',
      )
  }
  const supplier = snapshot.extractions.find((row) =>
    row.id.endsWith('/shared/S.tsx'),
  )
  assert(supplier)
  for (const row of shared)
    assert(
      classes(supplier.code).includes(row.name ?? ''),
      'shared generated supplier reference resolves',
    )
  const delivered =
    mode === 'serve'
      ? Object.entries(http)
          .filter(([path]) => /\.css(?:\?|$)/.test(path))
          .map(([, css]) => css)
          .join('\n')
      : readdirSync(resolve(root, 'out'))
          .filter((path) => path.endsWith('.css'))
          .map((path) => readFileSync(resolve(root, 'out', path), 'utf8'))
          .join('\n')
  if (lazy) {
    const supplier = snapshot.extractions.find((row) =>
      row.id.endsWith('/shared/P.tsx'),
    )
    assert(supplier)
    assert(supplier.map)
    const ids = [...supplier.code.matchAll(/devup-ui-(\d+)\.css/g)].map(
      (match) => Number(match[1]),
    )
    const local = ids.map((id) => wasm.getCss(id, false)).join('\n')
    const privateRules = declarations(
      single === 'single' ? snapshot.css : local,
    ).filter((row) => /border-color:teal|padding:36px/.test(row.value ?? ''))
    assert.equal(
      privateRules.length,
      2,
      'private supplier retains both declarations',
    )
    if (single !== 'single')
      assert(
        !/border-color:teal|padding:36px/.test(snapshot.css + disk),
        'unreachable lazy target cannot promote private supplier',
      )
    for (const row of privateRules) {
      assert.match(
        row.name ?? '',
        single === 'single' ? /^[a-z0-9]+$/ : /^[a-z0-9]+-[a-z0-9]+$/,
      )
      assert(classes(supplier.code).includes(row.name ?? ''))
      assert(
        delivered.includes(`.${row.name}{`),
        'delivered private supplier reference resolves',
      )
    }
    for (const name of ['src/unused-lazy.js', 'E.js']) {
      assert(
        !snapshot.extractions.some((row) => row.id.endsWith(`/${name}`)),
        'unreachable dynamic chain stays uncompiled',
      )
    }
    assert(
      snapshot.reservations.some((batch) =>
        batch.rows.some((row) => row[0].endsWith('/shared/P.tsx')),
      ),
    )
    const entry = snapshot.extractions.find((row) =>
      row.id.endsWith('/physical/A.tsx'),
    )
    assert(entry)
    assert(entry.dependencies.some((path) => path.endsWith('/shared/tone.js')))
    assert(entry.code.includes('"../shared/P"'))
    assert(entry.code.includes('"../shared/S"'))
  }
  for (const row of shared)
    assert(
      delivered.includes(`.${row.name}{`),
      'delivered shared declaration resolves',
    )
  if (mode === 'serve') {
    const sheets = Object.entries(http).filter(([path]) =>
      /devup-ui.*\.css/.test(path),
    )
    assert(sheets.length > 0, 'actual stylesheet HTTP responses')
    for (const row of shared)
      assert(
        sheets.some(([, css]) => css.includes(`.${row.name}{`)),
        'HTTP shared reference resolves',
      )
  }
}
module.exports = { checkHoist }
