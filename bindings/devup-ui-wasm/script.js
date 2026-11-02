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
const reflective =
  'const ret = Reflect.get(getObject(arg0), getObject(arg1)); return addHeapObject(ret);'
assert.deepEqual(
  caught.map((match) => body(match[1])).sort(),
  [
    'const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3)); return addHeapObject(ret);',
    'const ret = getObject(arg0).call(getObject(arg1)); return addHeapObject(ret);',
    'globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));',
    reflective,
    reflective,
    'const ret = getObject(arg0).next(); return addHeapObject(ret);',
  ].sort(),
)
assert.equal([...source.matchAll(/\bhandleError\(/g)].length, 7)
assert.match(
  source,
  /function handleError\(f, args\) \{\s*try \{\s*return f.apply\(this, args\);\s*\} catch \(e\) \{\s*wasm\.\w+\(addHeapObject\(e\)\);\s*\}\s*\}/,
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
assert.equal([...source.matchAll(/getObject\(arg0\)\[[^\r\n]+/g)].length, 3)
assert.match(source, /const ret = getObject\(arg0\)\.done;\s*return ret;/)
assert.match(
  source,
  /const ret = getObject\(arg0\)\.value;\s*return addHeapObject\(ret\);/,
)
assert.match(
  source,
  /function seedFileMap\(files\) \{\s*const ptr0 = passArrayJsValueToWasm0\(files, wasm\.\w+\);\s*const len0 = WASM_VECTOR_LEN;\s*wasm.seedFileMap\(ptr0, len0\);\s*\}/,
)
assert.match(
  source,
  /function passArrayJsValueToWasm0\(array, malloc\) \{\s*const ptr = malloc\(array.length \* 4, 4\) >>> 0;\s*const mem = getDataViewMemory0\(\);\s*for \(let i = 0; i < array.length; i\+\+\) \{\s*mem.setUint32\(ptr \+ 4 \* i, addHeapObject\(array\[i\]\), true\);\s*\}\s*WASM_VECTOR_LEN = array.length;\s*return ptr;\s*\}/,
)
rewrite(
  /Reflect.get\(getObject\(arg0\), getObject\(arg1\)\)/g,
  2,
  'Reflect.get(__devupBoundary.guard(getObject(arg0)), getObject(arg1))',
)
rewrite(
  /getObject\(arg0\)\[arg1 >>> 0\]/g,
  2,
  '__devupBoundary.guard(getObject(arg0), true)[arg1 >>> 0]',
)
for (const property of ['done', 'value'])
  rewrite(
    new RegExp(`getObject\\(arg0\\)\\.${property}`, 'g'),
    1,
    `__devupBoundary.guard(getObject(arg0)).${property}`,
  )
rewrite(
  /catch \(e\) \{\s*(wasm\.\w+\(addHeapObject\(e\)\);)/g,
  1,
  'catch (e) {\n        __devupBoundary.caught(e);\n        $1',
)
source = `const __devupBoundary = (${createBoundary.toString()})()\n${source}\n__devupBoundary.wrap(module.exports)\n`
writeFileSync('pkg/index.js', source, 'utf8')

// support mjs config
writeFileSync('pkg/package.json', JSON.stringify({}), 'utf8')
