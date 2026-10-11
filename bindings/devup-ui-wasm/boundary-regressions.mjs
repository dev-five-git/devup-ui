import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { fileURLToPath, pathToFileURL } from 'node:url'

const require = createRequire(import.meta.url)
const wasm = require('./pkg/index.js')
const filename = '/src/boundary.tsx'
const source = `import { Box } from '@devup-ui/react';
import { PRIMARY } from './tokens';
export const view = <Box bg='red' color={PRIMARY} />;`
const extract = (method, code = source) =>
  wasm[method](filename, code, '@devup-ui/react', 'df', true, false, false, {})

const loaderPath = require.resolve('@mdx-js/loader', {
  paths: [fileURLToPath(new URL('../../apps/landing/', import.meta.url))],
})
const compilerPath = createRequire(loaderPath).resolve('@mdx-js/mdx')
const { compile } = await import(pathToFileURL(compilerPath).href)
const markdown = `import { Box } from '@devup-ui/react'

# Heading

<Box bg="tomato">Hello</Box>
`
for (const jsx of [false, true]) {
  const compiled = String(
    await compile({ path: '/src/document.mdx', value: markdown }, { jsx }),
  )
  for (const extension of ['mdx', 'md']) {
    for (const method of ['codeExtract', 'codeExtractWithoutSourceMap']) {
      wasm.resetBuildState()
      const path = `/src/document.${extension}`
      assert.equal(wasm.hasDevupUI(path, compiled, '@devup-ui/react'), true)
      const output = wasm[method](
        path,
        compiled,
        '@devup-ui/react',
        'df',
        true,
        false,
        false,
        {},
      )
      assert.match(output.code, /className/)
      assert.doesNotMatch(output.code, /(?:_jsx\(\s*Box\b|<Box\b)/)
      assert.match(wasm.getCss(undefined, false), /background:tomato/)
      if (method === 'codeExtractWithoutSourceMap')
        assert.equal(output.map, undefined)
      else assert.deepEqual(JSON.parse(output.map).sources, [path])
      output.free()
    }
  }
}
for (const extension of ['mdx', 'md']) {
  wasm.resetBuildState()
  const path = `/src/raw.${extension}`
  assert.throws(
    () =>
      wasm.codeExtract(
        path,
        markdown,
        '@devup-ui/react',
        'df',
        true,
        false,
        false,
        {},
      ),
    (error) => {
      assert.ok(error instanceof Error)
      assert.ok(error.message.startsWith(`${path}:`))
      assert.match(error.message, /:\d+:\d+:/)
      return true
    },
  )
}
console.info(
  'Real MDX compiler regressions passed (default JSX calls and jsx:true).',
)

for (const jsx of [false, true]) {
  const markdown =
    "import { css } from '@emotion/react'\n\nexport const cls = css({ color: 'red' })\n\n<div className={cls} />\n"
  const compiled = String(
    await compile({ path: '/src/alias.mdx', value: markdown }, { jsx }),
  )
  wasm.resetBuildState()
  const output = wasm.codeExtract(
    '/src/alias.mdx',
    compiled,
    '@devup-ui/react',
    'df',
    true,
    false,
    false,
    { '@emotion/react': null },
  )
  assert.doesNotMatch(output.code, /@emotion\/react/)
  assert.match(wasm.getCss(undefined, false), /color:red/)
  output.free()
}

for (const method of ['codeExtract', 'codeExtractWithoutSourceMap']) {
  wasm.resetBuildState()
  const direct = extract(
    method,
    "import { Box } from '@devup-ui/react'; export const view = <Box bg='green' />;",
  )
  if (method === 'codeExtractWithoutSourceMap')
    assert.equal(direct.map, undefined)
  else assert.equal(typeof direct.map, 'string')
  direct.free()
  for (const result of [null, undefined]) {
    wasm.resetBuildState()
    wasm.setModuleResolver(() => result)
    const output = extract(method)
    assert.match(output.code, /--/)
    if (method === 'codeExtractWithoutSourceMap')
      assert.equal(output.map, undefined)
    else assert.equal(typeof output.map, 'string')
    output.free()
  }

  wasm.resetBuildState()
  wasm.setModuleResolver(() => ({
    path: '/src/tokens.ts',
    code: "export const PRIMARY = 'blue';",
  }))
  const resolved = extract(method)
  assert.deepEqual(resolved.dependencies, ['/src/tokens.ts'])
  assert.doesNotMatch(resolved.code, /--/)
  if (method === 'codeExtractWithoutSourceMap')
    assert.equal(resolved.map, undefined)
  resolved.free()

  const faults = [
    [
      () => {
        throw new Error('callback failure')
      },
      /callback failure/,
    ],
    [
      () => {
        throw 'string failure'
      },
      /string failure/,
    ],
    [
      () => ({
        get path() {
          throw new Error('path getter failure')
        },
      }),
      /path getter failure/,
    ],
    [
      () => ({
        path: '/src/tokens.ts',
        get code() {
          throw new Error('code getter failure')
        },
      }),
      /code getter failure/,
    ],
    [
      () =>
        new Proxy(
          {},
          {
            get() {
              throw new Error('reflection failure')
            },
          },
        ),
      /reflection failure/,
    ],
    [() => 7, /Reflect|reflect|object/i],
    [() => ({}), /path.*must be a string/],
    [() => ({ path: 7, code: '' }), /path.*must be a string/],
    [() => ({ path: '/src/tokens.ts', code: null }), /code.*must be a string/],
    [() => ({ path: '/src/tokens.ts', code: 7 }), /code.*must be a string/],
    [
      () => {
        throw new Proxy(
          {},
          {
            get() {
              throw new Error('hostile exception')
            },
          },
        )
      },
      /unprintable JavaScript exception/,
    ],
  ]
  for (const [resolver, cause] of faults) {
    wasm.resetBuildState()
    const initial = extract(
      method,
      "import { Box } from '@devup-ui/react'; export const view = <Box bg='green' />;",
    )
    initial.free()
    const before = wasm.exportSheet()
    wasm.setModuleResolver(resolver)
    assert.throws(
      () => extract(method),
      (error) => {
        assert.ok(error instanceof Error)
        assert.match(
          error.message,
          /^\/src\/boundary\.tsx:1:1: module resolver cannot use `\.\/tokens` at build time: /,
        )
        assert.match(error.message, cause)
        return true
      },
    )
    assert.equal(wasm.exportSheet(), before)
  }
}
wasm.resetBuildState()
console.info(
  'WASM boundary regressions passed (real generated JS/WASM, no mocks).',
)
