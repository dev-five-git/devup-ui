import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { pathToFileURL } from 'node:url'

import * as wasm from '@devup-ui/wasm'
import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import { withModuleResolver } from '../wasm'

const originalCwd = process.cwd()

describe('styled components another module defines', () => {
  let root: string

  const write = (path: string, code: string) => {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), code)
  }

  const extract = (file: string) =>
    wasm.codeExtractWithoutSourceMap(
      file,
      readFileSync(join(root, file), 'utf8'),
      '@devup-ui/react',
      '@devup-ui/react',
      true,
      false,
      false,
      { '@emotion/react': null, '@emotion/styled': 'styled' },
    )

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), 'devup-ui-imported-styled-'))
    write(
      'tsconfig.json',
      JSON.stringify({
        compilerOptions: { baseUrl: '.', paths: { '@/*': ['src/*'] } },
      }),
    )
    write(
      'src/Base.tsx',
      "import styled from '@emotion/styled'\nexport const Base = styled.div`color: red; padding: 4px;`\nexport const Dyn = styled.button`font-size: ${(p) => (p.big ? '20px' : '10px')};`\n",
    )
    write(
      'node_modules/@acme/ui/package.json',
      JSON.stringify({ name: '@acme/ui', exports: { '.': './src/index.js' } }),
    )
    write(
      'node_modules/@acme/ui/src/index.js',
      "import styled from '@emotion/styled'\nexport const Card = styled.section`border: 13px solid;`\n",
    )
    process.chdir(root)
    withModuleResolver(wasm)
  })

  afterEach(() => {
    process.chdir(originalCwd)
    rmSync(root, { recursive: true, force: true })
  })

  it('composes under the styles of the component using it', () => {
    write(
      'src/App.tsx',
      "import styled from '@emotion/styled'\nimport { Base, Dyn } from '@/Base'\nimport { Card } from '@acme/ui'\nexport const A = styled(Base).attrs({ role: 'note' })`color: blue;`\nexport const B = styled(Dyn)`margin: 0;`\nexport const C = styled(Card)`border: 17px dotted;`\nexport const W = Base.withComponent('section')\n",
    )
    const app = extract('src/App.tsx')
    expect(app.code).not.toContain('DevupAs = Base')
    expect(app.code).not.toContain('DevupAs = Dyn')
    expect(app.code).not.toContain('DevupAs = Card')
    expect(app.code).toContain('DevupAs = "div"')
    expect(app.code).toContain('DevupAs = "button"')
    expect(app.code).toContain('DevupAs = "section"')
    expect(app.code).toContain('role: "note"')
    expect([...app.dependencies].sort()).toEqual([
      'node_modules/@acme/ui/src/index.js',
      'src/Base.tsx',
    ])
    const css = wasm.getCss(null, false)
    expect(css).toContain('color:blue')
    expect(css).toContain('padding:4px')
    expect(css).toContain('border:17px dotted')
    expect(css).not.toContain('border:13px solid')
  })

  it('follows the definition when its module changes', () => {
    write(
      'src/App.tsx',
      "import styled from '@emotion/styled'\nimport { Base } from './Base'\nexport const A = styled(Base)`margin: 0;`\n",
    )
    const before = extract('src/App.tsx')
    write(
      'src/Base.tsx',
      "import styled from '@emotion/styled'\nexport const Base = styled.main`color: green;`\n",
    )
    const after = extract('src/App.tsx')
    expect(before.dependencies).toEqual(['src/Base.tsx'])
    expect(after.dependencies).toEqual(['src/Base.tsx'])
    expect(before.code).toContain('DevupAs = "div"')
    expect(after.code).toContain('DevupAs = "main"')
    expect(wasm.getCss(null, false)).toContain('color:green')
  })

  for (const singleCss of [false, true]) {
    for (const trigger of ['', "import { styled } from '@devup-ui/react'\n"]) {
      it(`retargets an imported dynamic component without a styling import (${singleCss}, ${!!trigger})`, async () => {
        // Given the same origin and resolver as the composing consumer.
        const source = `${trigger}import { Dyn } from '@/Base'\nexport const Retargeted = Dyn.withComponent('a')\n`
        const relevant = wasm.hasDevupUI(
          'src/Retargeted.tsx',
          source,
          '@devup-ui/react',
          { '@emotion/react': null, '@emotion/styled': 'styled' },
        )
        const compile = (file: string, code: string) =>
          wasm.codeExtractWithoutSourceMap(
            file,
            code,
            '@devup-ui/react',
            '@devup-ui/react',
            singleCss,
            false,
            false,
            { '@emotion/react': null, '@emotion/styled': 'styled' },
          )
        const transpiler = new Bun.Transpiler({
          loader: 'tsx',
          tsconfig: { compilerOptions: { jsx: 'react' } },
        })
        const preload = createRequire(import.meta.url).resolve(
          'bun-test-env-dom',
        )
        const testing = createRequire(preload).resolve('@testing-library/react')
        const react = pathToFileURL(
          createRequire(testing).resolve('react'),
        ).href
        const moduleUrl = (code: string) => {
          const javascript = transpiler
            .transformSync(
              code.replace(/^import ["'][^"']+\.css["'];?\s*$/gm, ''),
            )
            .replace(/from ["']react["']/g, `from ${JSON.stringify(react)}`)
          return `data:text/javascript;base64,${Buffer.from(`import * as React from ${JSON.stringify(react)};\n${javascript}`).toString('base64')}`
        }
        const base = moduleUrl(
          compile(
            'src/Base.tsx',
            readFileSync(join(root, 'src/Base.tsx'), 'utf8'),
          ).code,
        )

        // When the complete emitted consumer module is evaluated and rendered.
        const emitted = compile('src/Retargeted.tsx', source)
        const compiled = await import(
          moduleUrl(
            emitted.code.replace(
              /from ["']@\/Base["']/g,
              `from ${JSON.stringify(base)}`,
            ),
          )
        )
        const ref = { current: null }
        for (const big of [false, true]) {
          const element = compiled.Retargeted.render(
            { big, children: 'child' },
            ref,
          )

          // Then the retargeted element retains its ref, child and dynamic CSS.
          expect(element.type).toBe('a')
          expect(element.props.ref).toBe(ref)
          expect(element.props.children).toBe('child')
          const fileNumber = /devup-ui-(\d+)\.css$/.exec(
            emitted.cssFile ?? '',
          )?.[1]
          const css = wasm.getCss(fileNumber ? Number(fileNumber) : null, false)
          const declarations = element.props.className
            .split(' ')
            .flatMap((name: string) =>
              [...css.matchAll(new RegExp(`\\.${name}\\{([^}]+)\\}`, 'g'))].map(
                (match) => match[1],
              ),
            )
            .join(';')
          const fontSize = /font-size:([^;}]+)/
            .exec(declarations)?.[1]
            ?.replace(
              /var\((--[\w-]+)\)/g,
              (_match: string, variable: string) =>
                element.props.style[variable],
            )
          expect(fontSize).toBe(big ? '20px' : '10px')
        }
        expect(relevant).toBe(true)
      })
    }
  }

  it('selects a component by the class its own module gives it', () => {
    write(
      'src/App.tsx',
      "import styled from '@emotion/styled'\nimport { Base } from './Base'\nexport const P = styled.div`${Base} { margin: 7123px; }`\n",
    )
    extract('src/App.tsx')
    const base = extract('src/Base.tsx')
    const selector = /\.[\w-]+ \.([\w-]+)\{margin:7123px/.exec(
      wasm.getCss(null, false),
    )
    const marker = selector?.[1]
    expect(marker).toBeDefined()
    expect(base.code).toContain(marker as string)
  })

  it('keeps a component it cannot read a runtime component', () => {
    write(
      'src/Link.tsx',
      "import styled from '@emotion/styled'\nimport Link from 'next/link'\nexport const Nav = styled(Link)`color: red;`\n",
    )
    write(
      'src/App.tsx',
      "import styled from '@emotion/styled'\nimport { Nav } from './Link'\nexport const A = styled(Nav)`color: blue;`\n",
    )
    const app = extract('src/App.tsx')
    expect(app.code).toContain('DevupAs = Nav')
    expect(app.dependencies).toEqual(['src/Link.tsx'])
  })
})
