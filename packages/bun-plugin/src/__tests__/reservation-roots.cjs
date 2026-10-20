const assert = require('node:assert/strict')
const { readFileSync, writeFileSync } = require('node:fs')
const { relative, resolve } = require('node:path')
const { build } = require('bun')
const wasm = require('@devup-ui/wasm')

const [root, mode] = process.argv.slice(2)
process.chdir(root)
const batches = []
const extracts = []
const reserve = wasm.reserveClassSlots
wasm.reserveClassSlots = (...args) => {
  reserve(...args)
  batches.push(args[0])
}
const extract = wasm.codeExtract
wasm.codeExtract = (...args) => {
  const output = extract(...args)
  extracts.push([args[0], output.code])
  return output
}
const { DevupUI, register } = require('../../dist/register.cjs')

async function main() {
  let assets = {}
  if (mode === 'build') {
    const result = await build({
      entrypoints: ['./index.ts'],
      outdir: './out',
      plugins: [DevupUI()],
    })
    assert(result.success, result.logs.map(String).join('\n'))
    for (const output of result.outputs) {
      const bytes = await output.text()
      assets[relative(root, output.path)] = bytes
      writeFileSync(output.path, bytes)
    }
  } else {
    await register({ debug: mode === 'debug' })
    const module = await import(
      resolve(mode === 'outside' ? 'outside.ts' : 'src/A.ts')
    )
    assert.equal(typeof module.cls, 'string')
    assets = {
      css: readFileSync('df/devup-ui/devup-ui.css', 'utf8'),
      cls: module.cls,
    }
  }
  process.stdout.write(
    JSON.stringify({
      assets,
      batches,
      extracts,
      debug: wasm.isDebug(),
      classes: wasm.exportClassMap(),
      files: wasm.exportFileMap(),
      count: wasm.getUnreservedClassSlotCount(),
    }),
  )
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
