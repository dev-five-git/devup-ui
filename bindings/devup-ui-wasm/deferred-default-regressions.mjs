import assert from 'node:assert/strict'
import { createRequire } from 'node:module'

const wasm = createRequire(import.meta.url)('./pkg/index.js')
for (const extract of [wasm.codeExtract, wasm.codeExtractWithoutSourceMap]) {
  for (const request of ['./doc.mdx', './forward.js']) {
    // Given a real WASM extraction with a direct or transitive render-only read.
    wasm.resetStateForTesting()
    wasm.setModuleResolver((source) =>
      source === './forward.js'
        ? { path: '/forward.js', code: "export {default} from './doc.mdx'" }
        : undefined,
    )
    const source = `import Doc from '${request}'; export const view=<Doc/>;`
    // When either public extraction API returns unchanged source.
    const output = extract(
      '/page.tsx',
      source,
      '@devup-ui/react',
      'df',
      false,
      false,
      false,
      {},
      undefined,
      ['.mdx'],
    )
    try {
      // Then semantic root and cut obligations survive the real JS/WASM boundary.
      const records = JSON.parse(output.deferredDefaults)
      assert.equal(output.code, source)
      assert.equal(records.length, 1)
      assert.equal(records[0].extraction, '/page.tsx')
      assert.equal(records[0].request, request)
      assert.equal(
        records[0].cutImporter,
        request === './doc.mdx' ? '/page.tsx' : '/forward.js',
      )
      assert.equal(records[0].cutRequest, './doc.mdx')
      assert.equal(records[0].demand, 'render-only')
    } finally {
      output.free()
    }
  }
  for (const [newline, line] of [
    ['\n', 2],
    ['\r\n', 2],
    ['\r', 2],
    ['\u2028', 2],
    ['\u2029', 2],
    ['\r\n\u2028\r', 4],
  ]) {
    // Given real multiline source with a multibyte and surrogate-pair column.
    wasm.resetStateForTesting()
    wasm.setModuleResolver(() => undefined)
    const prefix = "const after='한😀'; import "
    const source = `const before='한😀';${newline}${prefix}Doc from './doc.mdx'; export const view=<Doc/>;`
    // When either public API crosses the actual WASM boundary.
    const output = extract(
      '/page.tsx',
      source,
      '@devup-ui/react',
      'df',
      false,
      false,
      false,
      {},
      undefined,
      ['.mdx'],
    )
    try {
      // Then authored coordinates use all JS line terminators and UTF-16 columns.
      const [record] = JSON.parse(output.deferredDefaults)
      assert.equal(record.line, line, JSON.stringify(newline))
      assert.equal(record.column, prefix.length + 1, JSON.stringify(newline))
    } finally {
      output.free()
    }
  }
}
console.info('Real WASM deferred-default boundaries passed: 16 cases')
