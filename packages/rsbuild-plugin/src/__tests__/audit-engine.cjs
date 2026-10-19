const { basename } = require('node:path')
/** @type {typeof import('@devup-ui/wasm')} */
const wasm = require('@devup-ui/wasm')
/** @type {unknown[]} */
const batches = []
/** @type {{name:string,code:string,map:string|undefined,before:number,after:number}[]} */
const extracts = []
const reserve = wasm.reserveClassSlots
wasm.reserveClassSlots = (...args) => {
  reserve(...args)
  batches.push({ rows: args[0], ...state() })
}
/** @type {() => void} */
let release = () => {}
const gate = new Promise((resolve) => {
  release = () => resolve(undefined)
})
const extract = wasm.codeExtract
wasm.codeExtract = (...args) => {
  const before = wasm.getUnreservedClassSlotCount()
  const output = extract(...args)
  extracts.push({
    name: args[0],
    code: output.code,
    map: output.map,
    before,
    after: wasm.getUnreservedClassSlotCount(),
  })
  if (basename(args[0]) === `${process.argv[4]?.[0]}.js`) release()
  return output
}

function state() {
  return {
    css: wasm.getCss(null, false),
    count: wasm.getUnreservedClassSlotCount(),
    classes: wasm.exportClassMap(),
    files: wasm.exportFileMap(),
  }
}

module.exports = { wasm, batches, extracts, gate, state }
