import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { pathToFileURL } from 'node:url'

const require = createRequire(
  new URL('../../apps/landing/package.json', import.meta.url),
)
const compiler = createRequire(require.resolve('@mdx-js/loader')).resolve(
  '@mdx-js/mdx',
)
const { compile } = await import(pathToFileURL(compiler).href)
const wasm = createRequire(import.meta.url)('./pkg/index.js')
const methods = ['codeExtract', 'codeExtractWithoutSourceMap']
let passed = 0

function check(name, action) {
  wasm.resetBuildState()
  action()
  passed += 1
  console.info(`PASS ${name}`)
}

function extract(method, filename, code, sourceType, aliases = {}) {
  return wasm[method](
    filename,
    code,
    '@devup-ui/react',
    'df',
    true,
    false,
    false,
    aliases,
    sourceType,
  )
}

const markdown =
  'import { Box } from "@devup-ui/react"\n\n<Box bg="tomato" />\n'
const aliasMarkdown =
  'import { css } from "@emotion/react"\n\nexport const cls = css({color: "blue"})\n\n<div className={cls} />\n'
for (const jsx of [false, true]) {
  const code = String(
    await compile(
      { path: '/src/page.mdown', value: markdown },
      { jsx, format: 'mdx' },
    ),
  )
  const alias = String(
    await compile(
      { path: '/src/alias.mdown', value: aliasMarkdown },
      { jsx, format: 'mdx' },
    ),
  )
  for (const method of methods) {
    for (const extension of ['md', 'mdx', 'mdown', 'ts']) {
      const filename = `/src/page.${extension}`
      check(`${method}:${jsx}:${extension}`, () => {
        const output = extract(
          method,
          filename,
          code,
          ['mdown', 'ts'].includes(extension) ? 'compiled-mdx' : undefined,
        )
        try {
          assert.match(wasm.getCss(undefined, false), /background:tomato/)
          assert.doesNotMatch(output.code, /(?:<Box\b|_jsx\(\s*Box\b)/)
          if (method === 'codeExtract')
            assert.deepEqual(JSON.parse(output.map).sources, [filename])
          else assert.equal(output.map, undefined)
        } finally {
          output.free()
        }
      })
    }
    check(`${method}:${jsx}:alias`, () => {
      const output = extract(
        method,
        '/src/alias.mdown',
        alias,
        'compiled-mdx',
        { '@emotion/react': null },
      )
      try {
        assert.match(wasm.getCss(undefined, false), /color:blue/)
      } finally {
        output.free()
      }
    })
    for (const reexport of [false, true]) {
      check(`${method}:${jsx}:imported:${reexport}`, () => {
        wasm.setModuleResolver((specifier) =>
          specifier === './entry' && reexport
            ? {
                path: '/src/reexport.js',
                code: "export {PRIMARY} from './tokens';",
              }
            : {
                path: '/src/tokens.mdown',
                code: `${code}\nexport const PRIMARY = 'blue';`,
                sourceType: 'compiled-mdx',
              },
        )
        const output = extract(
          method,
          '/src/App.tsx',
          'import {Box} from "@devup-ui/react"; import {PRIMARY} from "./entry"; export const view = <Box color={PRIMARY}/>;',
        )
        try {
          assert.match(wasm.getCss(undefined, false), /color:blue/)
          assert.doesNotMatch(wasm.getCss(undefined, false), /var\(--/)
          assert.ok(output.dependencies.includes('/src/tokens.mdown'))
        } finally {
          output.free()
        }
      })
    }
  }
}
for (const method of methods) {
  const stylesheetModule = String(
    await compile(
      { path: '/src/value.mdown', value: '# Heading' },
      { jsx: true, format: 'mdx' },
    ),
  )
  check(`${method}:stylesheet-import`, () => {
    wasm.setModuleResolver(() => ({
      path: '/src/value.mdown',
      code: `${stylesheetModule}\nexport const PRIMARY = 'blue';`,
      sourceType: 'compiled-mdx',
    }))
    const output = extract(
      method,
      '/src/theme.css.ts',
      'import {style} from "@vanilla-extract/css"; import {PRIMARY} from "./value"; export const cls = style({color: PRIMARY});',
      undefined,
      { '@vanilla-extract/css': null },
    )
    try {
      assert.match(wasm.getCss(undefined, false), /color:blue/)
    } finally {
      output.free()
    }
  })
  for (const value of ['invalid', '', 7, {}, true]) {
    check(`${method}:invalid-root:${JSON.stringify(value)}`, () => {
      const previous = extract(
        method,
        '/src/plain.js',
        'import {css} from "@devup-ui/react"; export const cls = css({color: "green"});',
      )
      previous.free()
      const sheet = wasm.exportSheet()
      assert.throws(
        () =>
          extract(method, '/src/plain.js', 'export const answer = 42;', value),
        /\/src\/plain\.js:1:1:.*source.*type/i,
      )
      assert.equal(wasm.exportSheet(), sheet)
    })
  }
  check(`${method}:unknown-fastpath`, () => {
    assert.throws(
      () => extract(method, '/src/plain.mdown', 'export const answer = 42;'),
      /\/src\/plain\.mdown:1:1:.*Unknown file extension/,
    )
  })
  for (const value of ['invalid', 7]) {
    check(`${method}:invalid-child:${value}`, () => {
      const initial = extract(
        method,
        '/src/App.tsx',
        'import {Box} from "@devup-ui/react"; export const view = <Box color="green"/>;',
      )
      initial.free()
      const sheet = wasm.exportSheet()
      wasm.setModuleResolver(() => ({
        path: '/src/token.js',
        code: "export const PRIMARY = 'blue';",
        sourceType: value,
      }))
      assert.throws(
        () =>
          extract(
            method,
            '/src/App.tsx',
            'import {Box} from "@devup-ui/react"; import {PRIMARY} from "./token"; export const view = <Box color={PRIMARY}/>;',
          ),
        /\/src\/App\.tsx:1:1:.*source.*type/i,
      )
      assert.equal(wasm.exportSheet(), sheet)
    })
  }
  check(`${method}:getter`, () => {
    wasm.setModuleResolver(() => ({
      path: '/src/token.js',
      code: "export const PRIMARY = 'blue';",
      get sourceType() {
        throw new Error('sourceType getter fault')
      },
    }))
    assert.throws(
      () =>
        extract(
          method,
          '/src/App.tsx',
          'import {Box} from "@devup-ui/react"; import {PRIMARY} from "./token"; export const view = <Box color={PRIMARY}/>;',
        ),
      /\/src\/App\.tsx:1:1:.*sourceType getter fault/,
    )
  })
  check(`${method}:root-mode-not-inherited`, () => {
    wasm.setModuleResolver(() => ({
      path: '/src/token.ts',
      code: "export enum Colors { PRIMARY = 'blue' }",
    }))
    const output = extract(
      method,
      '/src/App.mdown',
      'import {Box} from "@devup-ui/react"; import {Colors} from "./token"; export const view = <Box color={Colors.PRIMARY}/>;',
      'compiled-mdx',
    )
    try {
      assert.match(wasm.getCss(undefined, false), /color:blue/)
    } finally {
      output.free()
    }
  })
  check(`${method}:unknown-child`, () => {
    wasm.setModuleResolver(() => ({
      path: '/src/token.custom',
      code: "export const PRIMARY = 'blue';",
    }))
    assert.throws(
      () =>
        extract(
          method,
          '/src/App.tsx',
          'import {Box} from "@devup-ui/react"; import {PRIMARY} from "./token"; export const view = <Box color={PRIMARY}/>;',
        ),
      /\/src\/token\.custom:1:1:.*Unknown file extension/,
    )
  })
}
wasm.resetBuildState()
console.info(JSON.stringify({ passed }))
