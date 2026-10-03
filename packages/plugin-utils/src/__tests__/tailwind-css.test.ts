import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  findTailwindCss,
  TAILWIND_CSS_CANDIDATES,
  tailwindCssFiles,
  withTailwindCss,
} from '../tailwind-css'

let root: string

function write(file: string, content: string) {
  const path = join(root, file)
  mkdirSync(join(path, '..'), { recursive: true })
  writeFileSync(path, content)
}

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-tailwind-'))
})

afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

describe('findTailwindCss', () => {
  it('finds the conventional file that imports tailwindcss', () => {
    write(
      'src/app/globals.css',
      '@import "tailwindcss";\n@theme { --color-a: red; }',
    )
    const source = findTailwindCss({}, root)
    expect(source?.file).toBe(resolve(root, 'src/app/globals.css'))
    expect(source?.css).toContain('--color-a: red')
    expect(source?.files).toEqual([resolve(root, 'src/app/globals.css')])
  })

  it('skips a conventional file that does not import tailwindcss', () => {
    write('src/app/globals.css', 'body { margin: 0 }')
    write('src/index.css', "@import 'tailwindcss/theme';")
    expect(findTailwindCss({}, root)?.file).toBe(resolve(root, 'src/index.css'))
  })

  it('takes the file devup.json names, whatever it holds', () => {
    write('src/app/globals.css', '@import "tailwindcss";')
    write('styles/tw.css', '@theme { --color-b: blue }')
    const source = findTailwindCss({ tailwind: { css: 'styles/tw.css' } }, root)
    expect(source?.file).toBe(resolve(root, 'styles/tw.css'))
  })

  it('finds nothing when the named file is missing, when none imports tailwindcss, or when it is switched off', () => {
    expect(findTailwindCss({}, root)).toBeUndefined()
    expect(
      findTailwindCss({ tailwind: { css: 'missing.css' } }, root),
    ).toBeUndefined()
    write('src/app/globals.css', '@import "tailwindcss";')
    expect(findTailwindCss({ tailwind: { css: false } }, root)).toBeUndefined()
  })

  it('reads the local CSS files it imports, where they are imported', () => {
    write(
      'src/app/globals.css',
      '@import "tailwindcss";\n@import "./theme.css";\n@import "../shared/more.css" layer(base);\n@import "./missing.css";\n@import "https://x.test/a.css";\n.after {}',
    )
    write(
      'src/app/theme.css',
      '@theme { --color-c: red }\n@import "./theme.css";',
    )
    write('src/shared/more.css', '@utility u { a: b }')
    const source = findTailwindCss({}, root)
    expect(source?.files).toEqual([
      resolve(root, 'src/app/globals.css'),
      resolve(root, 'src/app/theme.css'),
      resolve(root, 'src/shared/more.css'),
    ])
    expect(source?.css).toBe(
      '@import "tailwindcss";\n@theme { --color-c: red }\n@import "./theme.css";\n@utility u { a: b }\n@import "./missing.css";\n@import "https://x.test/a.css";\n.after {}',
    )
  })

  it('stops following imports at a depth limit', () => {
    write('src/app/globals.css', '@import "tailwindcss";\n@import "./a0.css";')
    for (let index = 0; index < 12; index++) {
      write(
        `src/app/a${index}.css`,
        `.a${index} {}\n@import "./a${index + 1}.css";`,
      )
    }
    const source = findTailwindCss({}, root)
    expect(source?.files.length).toBe(9)
  })

  it('lists the conventional files', () => {
    expect(TAILWIND_CSS_CANDIDATES[0]).toBe('src/app/globals.css')
  })
})

describe('withTailwindCss', () => {
  it('adds the text of the Tailwind CSS to the theme', () => {
    write('src/index.css', '@import "tailwindcss";')
    expect(withTailwindCss({ colors: {} }, {}, root)).toEqual({
      colors: {},
      tailwindCss: '@import "tailwindcss";',
    })
  })

  it('leaves the theme alone without a Tailwind CSS', () => {
    const theme = { colors: {} }
    expect(withTailwindCss(theme, {}, root)).toBe(theme)
  })
})

describe('tailwindCssFiles', () => {
  it('lists every file read, none without a Tailwind CSS', () => {
    expect(tailwindCssFiles({}, root)).toEqual([])
    write('src/index.css', '@import "tailwindcss";\n@import "./t.css";')
    write('src/t.css', '')
    expect(tailwindCssFiles({}, root)).toEqual([
      resolve(root, 'src/index.css'),
      resolve(root, 'src/t.css'),
    ])
  })
})
