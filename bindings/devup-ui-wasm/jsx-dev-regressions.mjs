import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const project = createRequire(
  new URL('../../apps/landing/package.json', import.meta.url),
)
const compiler = createRequire(project.resolve('@mdx-js/loader'))
const { compile } = await import(
  pathToFileURL(compiler.resolve('@mdx-js/mdx')).href
)
const { parse } = compiler('acorn')
const wasm = createRequire(import.meta.url)('./pkg/index.js')
const markdown =
  'import { Box } from "@devup-ui/react"\n\n<Box as="section" bg="tomato"><Box p={2} /></Box>\n'
let passed = 0
let failed = 0

function structure(value) {
  return JSON.parse(
    JSON.stringify(value, (key, item) =>
      ['start', 'end', 'loc', 'raw'].includes(key) ? undefined : item,
    ),
  )
}

function calls(code) {
  const found = []
  function visit(node) {
    if (!node || typeof node !== 'object') return
    if (node.type === 'CallExpression' && node.callee.name === '_jsxDEV')
      found.push(node)
    for (const value of Object.values(node)) {
      if (Array.isArray(value)) value.forEach(visit)
      else visit(value)
    }
  }
  visit(parse(code, { ecmaVersion: 'latest', sourceType: 'module' }))
  return found
}

const root = mkdtempSync(join(tmpdir(), 'devup-jsxdev-'))
try {
  for (const extension of ['mdx', 'md', 'mdown']) {
    const filename = join(root, `development.${extension}`).replaceAll(
      '\\',
      '/',
    )
    writeFileSync(filename, markdown)
    const code = String(
      await compile(
        { path: filename, value: readFileSync(filename, 'utf8') },
        { development: true, jsx: false, format: 'mdx' },
      ),
    )
    const before = calls(code)
    const componentCalls = before.flatMap((call, index) =>
      call.arguments[0].name === 'Box' ? [index] : [],
    )
    assert.equal(componentCalls.length, 2)
    assert.match(code, /react\/jsx-dev-runtime/)
    for (const method of ['codeExtract', 'codeExtractWithoutSourceMap']) {
      wasm.resetBuildState()
      try {
        const output = wasm[method](
          filename,
          code,
          '@devup-ui/react',
          'df',
          true,
          false,
          false,
          {},
          extension === 'mdown' ? 'compiled-mdx' : undefined,
        )
        try {
          const after = calls(output.code)
          assert.equal(after.length, before.length)
          for (const [index, original] of before.entries()) {
            assert.equal(after[index].arguments.length, 6)
            assert.deepEqual(
              structure(after[index].callee),
              structure(original.callee),
            )
            assert.deepEqual(
              structure(after[index].arguments.slice(2)),
              structure(original.arguments.slice(2)),
            )
          }
          assert.match(wasm.getCss(undefined, false), /background:tomato/)
          assert.match(wasm.getCss(undefined, false), /padding:8px/)
          assert.doesNotMatch(output.code, /(?:<Box\b|_jsxDEV\(\s*Box\b)/)
          assert.equal(after[componentCalls[0]].arguments[0].value, 'section')
          assert.equal(after[componentCalls[1]].arguments[0].value, 'div')
          if (method === 'codeExtract')
            assert.deepEqual(JSON.parse(output.map).sources, [filename])
          else assert.equal(output.map, undefined)
        } finally {
          output.free()
        }
        passed += 1
        console.info(`PASS ${method}:${filename}:development:true`)
      } catch (error) {
        if (!(error instanceof Error)) throw error
        failed += 1
        console.error(`FAIL ${method}:${filename}: ${error.message}`)
      }
    }
  }
} finally {
  wasm.resetBuildState()
  rmSync(root, { recursive: true, force: true })
}
console.info(JSON.stringify({ passed, failed }))
process.exitCode = failed ? 1 : 0
