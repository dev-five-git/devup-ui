import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'

import createBoundary from '../argument-validation.cjs'

let source = readFileSync('pkg/index.js', 'utf8')
const rewrite = (pattern, count, replacement) => {
  assert.equal([...source.matchAll(pattern)].length, count, String(pattern))
  source = source.replace(pattern, replacement)
}
const caught = [
  ...source.matchAll(
    /return handleError\(function \([^)]*\) \{([\s\S]*?)\}, arguments\);/g,
  ),
]
const body = (text) => text.replace(/\s+/g, ' ').trim()
const reflective = 'const ret = Reflect.get(arg0, arg1); return ret;'
assert.deepEqual(
  caught.map((match) => body(match[1])).sort(),
  [
    'const ret = arg0.call(arg1, arg2, arg3); return ret;',
    'const ret = arg0.call(arg1); return ret;',
    'globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));',
    reflective,
    reflective,
    'const ret = arg0.next(); return ret;',
  ].sort(),
)
assert.equal([...source.matchAll(/\bhandleError\(/g)].length, 7)
assert.match(
  source,
  /function handleError\(f, args\) \{\s*try \{\s*return f.apply\(this, args\);\s*\} catch \(e\) \{\s*const idx = addToExternrefTable0\(e\);\s*wasm\.\w+\(idx\);\s*\}\s*\}/,
)
for (const operation of ['begin', 'end', 'error', 'fail'])
  assert.equal(
    [...source.matchAll(new RegExp(`__devupBoundary\\.${operation}\\(`, 'g'))]
      .length,
    1,
  )
assert.equal([...source.matchAll(/__devupBoundary/g)].length, 4)
const names = [...source.matchAll(/^exports\.(\w+) = \1;/gm)]
assert.equal(names.length, 29)
assert.equal(new Set(names.map((match) => match[1])).size, 29)
assert.equal([...source.matchAll(/^exports\./gm)].length, 29)
assert.equal(source.includes('module.exports'), false)
for (const name of ['codeExtract', 'codeExtractWithoutSourceMap']) {
  const signatures = [
    ...source.matchAll(new RegExp(`function ${name}\\(([^)]*)\\)`, 'g')),
  ]
  assert.equal(signatures.length, 1)
  assert.equal(signatures[0][1].split(',').length, 8)
}
assert.equal([...source.matchAll(/\bpassArrayJsValueToWasm0\(/g)].length, 2)
assert.equal([...source.matchAll(/\barg0\[[^\r\n]+/g)].length, 3)
assert.match(source, /const ret = arg0\.done;\s*return ret;/)
assert.match(source, /const ret = arg0\.value;\s*return ret;/)
assert.match(
  source,
  /function seedFileMap\(files\) \{\s*const ptr0 = passArrayJsValueToWasm0\(files, wasm\.\w+\);\s*const len0 = WASM_VECTOR_LEN;\s*wasm.seedFileMap\(ptr0, len0\);\s*\}/,
)
assert.match(
  source,
  /function passArrayJsValueToWasm0\(array, malloc\) \{\s*const ptr = malloc\(array.length \* 4, 4\) >>> 0;\s*for \(let i = 0; i < array.length; i\+\+\) \{\s*const add = addToExternrefTable0\(array\[i\]\);\s*getDataViewMemory0\(\)\.setUint32\(ptr \+ 4 \* i, add, true\);\s*\}\s*WASM_VECTOR_LEN = array.length;\s*return ptr;\s*\}/,
)
rewrite(
  /Reflect.get\(arg0, arg1\)/g,
  2,
  'Reflect.get(__devupBoundary.guard(arg0), arg1)',
)
rewrite(
  /\barg0\[arg1 >>> 0\]/g,
  2,
  '__devupBoundary.guard(arg0, true)[arg1 >>> 0]',
)
for (const property of ['done', 'value'])
  rewrite(
    new RegExp(`\\barg0\\.${property}\\b`, 'g'),
    1,
    `__devupBoundary.guard(arg0).${property}`,
  )
rewrite(
  /catch \(e\) \{\s*(const idx = addToExternrefTable0\(e\);)/g,
  1,
  'catch (e) {\n        __devupBoundary.caught(e);\n        $1',
)
source = `const __devupBoundary = (${createBoundary.toString()})()\n${source}\n__devupBoundary.wrap(module.exports)\n`
writeFileSync('pkg/index.js', source, 'utf8')

// support mjs config
writeFileSync('pkg/package.json', JSON.stringify({}), 'utf8')
