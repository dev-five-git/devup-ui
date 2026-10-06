import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const wasm = require('../pkg/index.js')
const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789_'
const size = (value) => {
  const bytes = Buffer.alloc(8)
  bytes.writeBigUInt64BE(BigInt(value))
  return bytes
}
const text = (value) =>
  Buffer.concat([size(Buffer.byteLength(value)), Buffer.from(value)])
const atom = (property, value) =>
  Buffer.concat([
    Buffer.from([1, 1, 0]),
    text(property),
    Buffer.from([1]),
    text(value),
    Buffer.from([0, 255, 0, 0]),
  ])
const fingerprint = (descriptor) => {
  const first80 = createHash('sha256')
    .update(descriptor)
    .digest()
    .subarray(0, 10)
  let value = BigInt(`0x${first80.toString('hex')}`)
  let result = ''
  for (let digit = 0; digit < 16; digit++) {
    result = alphabet[Number(value % 37n)] + result
    value /= 37n
  }
  assert.equal(value, 0n)
  return result
}
const compile = (file, code) =>
  wasm.codeExtract(file, code, '@devup-ui/react', 'df', true, false, false, {})

wasm.resetBuildState()
wasm.setDebug(false)
if (process.argv[2] === 'goldens') {
  const source =
    "import {css,keyframes} from '@devup-ui/react'; export const x=css({color:'red',fontFamily:'abcdefghijklmnopqrstuvwx'});export const k=keyframes({from:{opacity:0},to:{opacity:1}});"
  const output = compile('golden.tsx', source)
  const family =
    'OH' + fingerprint(atom('font-family', 'abcdefghijklmnopqrstuvwx'))
  assert.ok(output.code.includes('OLcolor-vred'))
  assert.ok(output.code.includes(family))
  const frames = Buffer.concat([
    Buffer.from([1, 3]),
    size(2),
    text('from'),
    size(1),
    text('opacity'),
    text('0'),
    text('to'),
    size(1),
    text('opacity'),
    text('1'),
  ])
  const animation = 'KH' + fingerprint(frames)
  assert.ok(output.code.includes(animation))
  assert.ok(wasm.getCss(null, false).includes('@keyframes ' + animation))
  const sheet = JSON.parse(wasm.exportSheet())
  assert.deepEqual(sheet.names[family].descriptor, [
    ...atom('font-family', 'abcdefghijklmnopqrstuvwx'),
  ])
  assert.deepEqual(sheet.names[animation].descriptor, [...frames])
  output.free()
} else if (process.argv[2] === 'compact-sources') {
  const source =
    '// filler\n'.repeat(10_240) +
    "import {Box} from '@devup-ui/react';\nexport const View=(p)=><Box color={p.c} w={p.w}/>;\n"
  const root = '---SUH' + fingerprint(Buffer.from(source))
  const run = (received) => {
    wasm.resetBuildState()
    wasm.setDebug(false)
    wasm.registerTheme({})
    const output = compile('/real/large.tsx', received)
    const variables = output.code.match(/---SUH[a-z0-9_]{16}-[a-z0-9_]+/gu)
    assert.equal(variables.length, 2)
    assert.ok(variables.every((name) => name.startsWith(root + '-')))
    assert.ok(variables.every((name) => name.length <= 38))
    const css = wasm.getCss(null, false)
    assert.ok(variables.every((name) => css.includes(name)))
    const sheet = JSON.parse(wasm.exportSheet())
    assert.equal(sheet.names[root].content, source)
    assert.deepEqual(sheet.names[root].descriptor, [...Buffer.from(source)])
    const result = {
      code: output.code,
      css: output.css,
      map: output.map,
      cssFile: output.cssFile,
      updatedBaseStyle: output.updatedBaseStyle,
      dependencies: output.dependencies,
      completeCss: css,
    }
    output.free()
    wasm.resetBuildState()
    wasm.importSheet(sheet)
    const restored = compile('/real/other.tsx', source)
    assert.equal(restored.code, result.code)
    assert.equal(wasm.getCss(null, false), css)
    restored.free()
    return result
  }
  const baseline = run(source)
  for (const received of [
    source,
    source.replaceAll('\n', '\r\n'),
    '\uFEFF' + source,
    '\uFEFF' + source.replaceAll('\n', '\r\n'),
  ]) {
    assert.deepEqual(run(received), baseline)
  }
} else if (process.argv[2] === 'cache-conflict') {
  compile(
    'a.tsx',
    "import {css} from '@devup-ui/react';\nexport const x=css({color:'blue'});",
  ).free()
  compile(
    'b.tsx',
    "import {globalCss} from '@devup-ui/react'; globalCss({body:{padding:'11px'}});",
  ).free()
  const sheet = JSON.parse(wasm.exportSheet())
  sheet.names['OLcolor-vred'] = sheet.names['OLcolor-vblue']
  wasm.importSheet(sheet)
  const before = wasm.exportSheet()
  const css = wasm.getCss(null, false)
  assert.throws(
    () =>
      compile(
        'b.tsx',
        "import {css} from '@devup-ui/react';\nexport const x=css({color:'red'});",
      ),
    (error) => {
      assert.ok(error instanceof Error)
      assert.ok(error.message.includes('a.tsx:2:27:'))
      assert.ok(error.message.includes('b.tsx:2:27:'))
      assert.ok(error.message.includes('OLcolor-vred'))
      return true
    },
  )
  assert.equal(wasm.exportSheet(), before)
  assert.equal(wasm.getCss(null, false), css)
} else {
  assert.fail('unknown fixture mode')
}
wasm.resetBuildState()
