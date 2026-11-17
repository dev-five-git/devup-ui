import { readdirSync, readFileSync } from 'node:fs'
import { join, relative } from 'node:path'

import { describe, expect, it } from 'bun:test'

// Docs examples checked against what the build emits, so they cannot drift.
//
// An example opts in with an MDX comment on the line before its ```tsx fence:
//
//   {/* conformance */}         the example alone
//   {/* conformance: theme */}  with the `theme` of the nearest ```json above
//
// The next ```css fence is the CSS the example must emit, rule for rule (in
// any order; the theme layer `@layer t` only when the fence shows it). A
// ```tsx fence in between is the rendered markup: its `className` values must
// match the output's, in order. An example without an import gets one for the
// components it uses, and one that starts with `<` is wrapped in a fragment.
//
// The prop and selector tables are checked row by row as well.

const wasm = await import('../../bindings/devup-ui-wasm/pkg/index.js')

const PACKAGE = '@devup-ui/react'
const docsRoot = join(
  import.meta.dirname,
  '../../apps/landing/src/app/(detail)/docs',
)

/** Pages whose examples are opted in; a page losing its markers fails. */
const CHECKED_PAGES = [
  'api/box/page.mdx',
  'api/center/page.mdx',
  'api/css/page.mdx',
  'api/flex/page.mdx',
  'api/grid/page.mdx',
  'api/group-selector/page.mdx',
  'api/input/page.mdx',
  'api/text/page.mdx',
  'api/v-stack/page.mdx',
  'devup/breakpoints/page.mdx',
  'devup/colors/page.mdx',
  'devup/length/page.mdx',
  'devup/shadow/page.mdx',
]

interface Fence {
  lang: string
  code: string
  line: number
  marker: string | null
}

interface Example {
  page: string
  line: number
  source: string
  theme: unknown
  rendered: string | null
  css: string
}

function mdxFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name)
    if (entry.isDirectory()) return mdxFiles(path)
    return entry.name.endsWith('.mdx') ? [path] : []
  })
}

function fences(mdx: string): Fence[] {
  const lines = mdx.replaceAll('\r\n', '\n').split('\n')
  const found: Fence[] = []
  for (let index = 0; index < lines.length; index++) {
    const open = /^```([\w-]*)/.exec(lines[index])
    if (!open) continue
    const start = index
    const body: string[] = []
    while (++index < lines.length && !lines[index].startsWith('```'))
      body.push(lines[index])
    let previous = start - 1
    while (previous >= 0 && lines[previous].trim() === '') previous--
    const match = /^\{\/\*\s*conformance(?::\s*(\w+))?\s*\*\/\}$/.exec(
      lines[previous]?.trim() ?? '',
    )
    found.push({
      lang: open[1],
      code: body.join('\n'),
      line: start + 1,
      marker: match ? (match[1] ?? '') : null,
    })
  }
  return found
}

function examples(page: string): Example[] {
  const all = fences(readFileSync(join(docsRoot, page), 'utf8'))
  return all.flatMap((fence, index) => {
    if (fence.marker === null) return []
    if (fence.lang !== 'tsx')
      throw new Error(`${page}:${fence.line}: conformance marks a tsx fence`)
    let theme: unknown = {}
    if (fence.marker === 'theme') {
      const json = all
        .slice(0, index)
        .reverse()
        .find((item) => item.lang === 'json')
      if (!json)
        throw new Error(`${page}:${fence.line}: no json fence above for theme`)
      theme = (JSON.parse(json.code) as { theme: unknown }).theme
    } else if (fence.marker !== '') {
      throw new Error(`${page}:${fence.line}: unknown marker ${fence.marker}`)
    }
    const [next, after] = all.slice(index + 1)
    const rendered = next?.lang === 'tsx' ? next : null
    const css = rendered ? after : next
    if (css?.lang !== 'css')
      throw new Error(`${page}:${fence.line}: no css fence after the example`)
    return [
      {
        page,
        line: fence.line,
        source: fence.code,
        theme,
        rendered: rendered?.code ?? null,
        css: css.code,
      },
    ]
  })
}

function standalone(source: string): string {
  let code = source.trim()
  if (code.startsWith('<'))
    code = `export const Example = (\n<>\n${code}\n</>\n)`
  if (!code.includes(`from '${PACKAGE}'`)) {
    const tags = [
      ...new Set([...code.matchAll(/<([A-Z]\w*)/g)].map((m) => m[1])),
    ]
    code = `import { ${tags.sort().join(', ')} } from '${PACKAGE}'\n${code}`
  }
  return code
}

function extract(source: string, theme: unknown) {
  wasm.resetStateForTesting()
  wasm.setDebug(false)
  wasm.registerTheme(theme)
  using output = wasm.codeExtract(
    'docs/Example.tsx',
    standalone(source),
    PACKAGE,
    'df',
    true,
    false,
    false,
    {},
  )
  return { code: output.code, css: output.css ?? '' }
}

function compact(text: string): string {
  return text
    .replace(/\s+/g, ' ')
    .replace(/\s*([,>+~(){}])\s*/g, '$1')
    .replace(/\[([\w-]+)=(['"])([\w-]+)\2\]/g, '[$1=$3]')
    .trim()
}

function value(text: string): string {
  return compact(text)
    .replace(/#[\da-f]{3,8}\b/gi, (hex) => hex.toLowerCase())
    .replace(/(^|[\s,(])0\.(\d)/g, '$1.$2')
}

function declarations(body: string): string {
  return body
    .split(';')
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => {
      const colon = item.indexOf(':')
      return `${item.slice(0, colon).trim().toLowerCase()}:${value(item.slice(colon + 1))}`
    })
    .sort()
    .join(';')
}

/** Flatten CSS into `wrappers selector{sorted declarations}` strings. */
function rules(css: string, wrappers: string[] = []): string[] {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, '')
  const out: string[] = []
  let index = 0
  while (index < text.length) {
    const open = text.indexOf('{', index)
    if (open === -1) break
    const semicolon = text.indexOf(';', index)
    if (semicolon !== -1 && semicolon < open) {
      index = semicolon + 1
      continue
    }
    let depth = 1
    let close = open + 1
    for (; depth > 0 && close < text.length; close++) {
      if (text[close] === '{') depth++
      else if (text[close] === '}') depth--
    }
    const prelude = compact(text.slice(index, open)).replace(/\s*:\s*/g, ':')
    const body = text.slice(open + 1, close - 1)
    if (prelude.startsWith('@'))
      out.push(...rules(body, [...wrappers, prelude]))
    else out.push(`${[...wrappers, prelude].join(' ')}{${declarations(body)}}`)
    index = close
  }
  return out
}

/** The theme layer is shown only by examples about it. */
function emitted(css: string, documented: string[]): string[] {
  const all = rules(css)
  if (documented.some((rule) => rule.startsWith('@layer t '))) return all
  return all.filter((rule) => !rule.startsWith('@layer t '))
}

function classNames(code: string): string[] {
  return [...code.matchAll(/className=\{?"([^"]*)"/g)].map((match) => match[1])
}

describe('documented examples emit the CSS they show', () => {
  const found = CHECKED_PAGES.flatMap(examples)

  it('every checked page opts in at least one example', () => {
    expect(
      CHECKED_PAGES.filter(
        (page) => !found.some((example) => example.page === page),
      ),
    ).toEqual([])
    const marked = mdxFiles(docsRoot)
      .map((path) => relative(docsRoot, path).replaceAll('\\', '/'))
      .filter((page) =>
        fences(readFileSync(join(docsRoot, page), 'utf8')).some(
          (fence) => fence.marker !== null,
        ),
      )
    expect(marked.filter((page) => !CHECKED_PAGES.includes(page))).toEqual([])
  })

  it.each(
    found.map(
      (example) => [`${example.page}:${example.line}`, example] as const,
    ),
  )('%s', (_, example) => {
    const output = extract(example.source, example.theme)
    const documented = rules(example.css).sort()
    expect(emitted(output.css, documented).sort()).toEqual(documented)
    if (example.rendered !== null)
      expect(classNames(output.code)).toEqual(classNames(example.rendered))
  })
})

/** `[names, targets]` of each two-column table row, splitting `a, b` names. */
function tableRows(page: string): [string, string[]][] {
  const mdx = readFileSync(join(docsRoot, page), 'utf8')
  return [...mdx.matchAll(/<tr>([\s\S]*?)<\/tr>/g)].flatMap((row) => {
    const cells = [...row[1].matchAll(/<td>([\s\S]*?)<\/td>/g)].map((cell) =>
      [...cell[1].matchAll(/<code>([^<]+)<\/code>/g)].map((code) => code[1]),
    )
    if (cells.length !== 2 || cells[0].length !== 1 || cells[1].length === 0)
      return []
    return cells[0][0]
      .split(',')
      .map((name): [string, string[]] => [name.trim(), cells[1]])
  })
}

function properties(css: string): string[] {
  return rules(css)
    .flatMap((rule) =>
      rule
        .slice(rule.indexOf('{') + 1, -1)
        .split(';')
        .map((item) => item.slice(0, item.indexOf(':'))),
    )
    .sort()
}

describe('the style props table', () => {
  const rows = tableRows('api/style-props/page.mdx')

  it('lists the props', () => {
    expect(rows.length).toBeGreaterThan(300)
  })

  it.each(rows)('%s sets %p', (prop, documented) => {
    const output = extract(`<Box ${prop}="inherit" />`, {})
    expect(properties(output.css)).toEqual([...documented].sort())
  })
})

describe('the selector table', () => {
  const rows = tableRows('api/selector/page.mdx')

  it('lists the selectors', () => {
    expect(rows.length).toBeGreaterThan(20)
  })

  it.each(rows)('%s selects %p', (selector, [target]) => {
    const output = extract(`<Box ${selector}={{ color: 'red' }} />`, {})
    // A pseudo-class or element follows the class; anything else is a media query.
    const expected = target.startsWith(':')
      ? `.a${target}{color:red}`
      : `@media ${target}{.a{color:red}}`
    expect(rules(output.css)).toEqual(rules(expected))
  })
})

describe('every code example parses as a standalone module', () => {
  // A fence holding several files redeclares what it imports: split it.
  const transpiler = new Bun.Transpiler({ loader: 'tsx' })
  const pages = mdxFiles(docsRoot).map((path) =>
    relative(docsRoot, path).replaceAll('\\', '/'),
  )

  it.each(pages)('%s', (page) => {
    const failures = fences(readFileSync(join(docsRoot, page), 'utf8'))
      .filter((fence) => ['js', 'jsx', 'ts', 'tsx'].includes(fence.lang))
      .flatMap((fence) => {
        try {
          transpiler.transformSync(fence.code)
          return []
        } catch (error) {
          return [`${page}:${fence.line}: ${String(error)}`]
        }
      })
    expect(failures).toEqual([])
  })
})
