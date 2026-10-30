const assert = require('node:assert/strict')

const colors = {
  maroon: '#800000',
  cyan: '#00ffff',
  aqua: '#00ffff',
  white: '#ffffff',
  goldenrod: '#daa520',
  rebeccapurple: '#663399',
}
function normalize(value) {
  const lower = value.trim().toLowerCase()
  if (colors[lower]) return colors[lower]
  if (/^#[\da-f]{3}$/.test(lower))
    return '#' + [...lower.slice(1)].map((digit) => digit + digit).join('')
  const rgb = lower.match(
    /^rgba?\(\s*(\d+)[,\s]+(\d+)[,\s]+(\d+)(?:\s*[,/]\s*1)?\s*\)$/,
  )
  return rgb
    ? '#' +
        rgb
          .slice(1)
          .map((part) => Number(part).toString(16).padStart(2, '0'))
          .join('')
    : lower
}
function rules(css, parse) {
  const result = new Map()
  parse(css).walkRules((rule) => {
    for (const selector of rule.selectors) {
      const declarations = result.get(selector) || new Map()
      rule.walkDecls((decl) =>
        declarations.set(decl.prop, normalize(decl.value)),
      )
      result.set(selector, declarations)
    }
  })
  return result
}
function declarationProof(parsed, selectors, expected) {
  const declarations = new Map(
    selectors.flatMap((selector) => [...(parsed.get(selector) || [])]),
  )
  for (const [property, value] of Object.entries(expected))
    assert.equal(
      declarations.get(property),
      normalize(value),
      `${selectors.join(' ')} missing/wrong ${property}; actual ${JSON.stringify([...declarations])}`,
    )
  return [...declarations].sort(([a], [b]) => a.localeCompare(b))
}
function elementProof(html, parsed, proof) {
  const tag = [...html.matchAll(/<[a-z][^>]*>/gi)]
    .map(([value]) => value)
    .find((value) => value.includes(`id="${proof.id}"`))
  assert.ok(tag, `missing SSR element ${proof.id}`)
  const classes = tag.match(/\bclass="([^"]+)"/)?.[1].split(/\s+/)
  assert.ok(classes?.length, `missing actual classes for ${proof.id}`)
  for (const name of classes) {
    assert.match(name, /^[\w-]+$/)
    if (name.includes('--')) continue
    assert.ok(
      parsed.get(`.${name}`)?.size,
      `${proof.id}: actual class .${name} has no linked declarations`,
    )
  }
  return {
    id: proof.id,
    classes: [...classes].sort(),
    declarations: declarationProof(
      parsed,
      classes.map((name) => `.${name}`),
      proof.expected,
    ),
  }
}
async function documentProof(base, parse, page) {
  const response = await fetch(new URL(page.path, base), {
    signal: AbortSignal.timeout(10000),
  })
  assert.equal(response.status, 200, `${page.path} HTML status`)
  assert.match(response.headers.get('content-type') || '', /text\/html/)
  const html = await response.text()
  const links = [...html.matchAll(/<link\b[^>]*>/g)]
    .map(([tag]) => tag)
    .filter((tag) => /\brel="stylesheet"/.test(tag))
    .map((tag) => tag.match(/\bhref="([^"]+)"/)?.[1].replaceAll('&amp;', '&'))
  assert.ok(links.length > 0, `${page.path} must link CSS`)
  const sheets = []
  for (const link of [...new Set(links)]) {
    assert.ok(link, 'stylesheet link must have href')
    const response = await fetch(new URL(link, base), {
      signal: AbortSignal.timeout(10000),
    })
    assert.equal(response.status, 200, `${page.path}: stylesheet ${link}`)
    assert.match(response.headers.get('content-type') || '', /text\/css/)
    const css = await response.text()
    assert.ok(css.trim().length > 0, `${link} empty CSS`)
    sheets.push({ url: link, css })
  }
  const parsed = rules(sheets.map(({ css }) => css).join('\n'), parse)
  const elements = [
    ...page.elements,
    { id: 'layout-proof', expected: { color: 'white' } },
  ].map((proof) => elementProof(html, parsed, proof))
  const body = declarationProof(parsed, ['body'], {
    background: '#040',
    color: '#248',
  })
  const theme = [...parsed.values()].some(
    (declarations) => declarations.get('--proof') === '#123456',
  )
  assert.ok(theme, `${page.path} linked theme --proof missing`)
  return { path: page.path, elements, body, theme, sheets }
}
module.exports = async function deliveryHttp(base, parse) {
  const pages = [
    {
      path: '/',
      elements: [
        {
          id: 'server-proof',
          expected: { background: 'maroon', padding: '13px' },
        },
        { id: 'client-proof', expected: { color: 'cyan' } },
        {
          id: 'external-proof',
          expected: { color: 'rebeccapurple', 'margin-left': '17px' },
        },
      ],
    },
    {
      path: '/client',
      elements: [{ id: 'client-proof', expected: { color: 'cyan' } }],
    },
    {
      path: '/edge-page',
      elements: [
        {
          id: 'edge-page-proof',
          expected: { color: 'goldenrod', padding: '19px' },
        },
      ],
    },
  ]
  const documents = []
  for (const page of pages)
    documents.push(await documentProof(base, parse, page))
  console.info('HTTP ' + JSON.stringify(documents))
  // Compare semantic stylesheet contents/class bindings, never Next build-id URLs/manifests.
  return documents.map(({ sheets, ...document }) => ({
    ...document,
    css: sheets.map(({ css }) => css).sort(),
  }))
}
module.exports.rules = rules
module.exports.declarationProof = declarationProof
