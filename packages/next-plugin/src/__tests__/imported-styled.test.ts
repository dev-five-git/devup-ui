import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

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
