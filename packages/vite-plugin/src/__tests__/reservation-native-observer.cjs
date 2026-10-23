const assert = require('node:assert/strict')
const wasm = require('@devup-ui/wasm')

/** @typedef {readonly [string, string, string]} Row */
/** @typedef {Readonly<Record<string, string | boolean>>} Aliases */
/** @typedef {[readonly Row[], string, boolean, boolean, boolean, Aliases]} ReserveArgs */
/** @typedef {[string, string, string, string, boolean, boolean, boolean, Aliases]} ExtractArgs */
/** @typedef {{id:string,input:string,code:string,css?:string,map?:string,cssFile?:string,dependencies:readonly string[],updatedBaseStyle:boolean,before:number,after:number}} Extraction */
/** @typedef {{rows:readonly Row[],flags:readonly unknown[],classMap:string,fileMap:string}} Reservation */

/** Observe the real facade, materializing every getter before returning its live Output. */
function observe() {
  /** @type {string[]} */
  const events = []
  /** @type {Reservation[]} */
  const reservations = []
  /** @type {Extraction[]} */
  const extractions = []
  const reserve = wasm.reserveClassSlots
  /** @param {ReserveArgs} args */
  wasm.reserveClassSlots = (...args) => {
    const sheet = wasm.exportSheet()
    reserve(...args)
    assert.equal(
      wasm.exportSheet(),
      sheet,
      'reservation publishes no scratch CSS',
    )
    reservations.push({
      rows: args[0],
      flags: args.slice(1),
      classMap: wasm.exportClassMap(),
      fileMap: wasm.exportFileMap(),
    })
    events.push('reserve')
  }
  const extract = wasm.codeExtract
  /** @type {((id:string)=>void) | undefined} */
  let onExtract
  /** @param {ExtractArgs} args */
  wasm.codeExtract = (...args) => {
    const before = wasm.getUnreservedClassSlotCount()
    const output = extract(...args)
    const { code, css, map, cssFile, dependencies, updatedBaseStyle } = output
    extractions.push({
      id: args[0],
      input: args[1],
      code,
      ...(css === undefined ? {} : { css }),
      ...(map === undefined ? {} : { map }),
      ...(cssFile === undefined ? {} : { cssFile }),
      dependencies,
      updatedBaseStyle,
      before,
      after: wasm.getUnreservedClassSlotCount(),
    })
    events.push(`extract:${args[0]}`)
    onExtract?.(args[0])
    return output
  }
  return {
    events,
    reservations,
    extractions,
    /** @param {(id:string)=>void} callback */
    onExtract(callback) {
      onExtract = callback
    },
    snapshot() {
      return {
        events,
        reservations,
        extractions,
        count: wasm.getUnreservedClassSlotCount(),
        css: wasm.getCss(null, false),
        classMap: wasm.exportClassMap(),
        fileMap: wasm.exportFileMap(),
      }
    },
  }
}
module.exports = { observe }
