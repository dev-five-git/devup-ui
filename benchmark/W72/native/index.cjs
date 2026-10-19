'use strict'

const native = require('./devup_ui_w72_native.node')

// serde-wasm-bindgen accepts JS Maps, iterables and undefined Option values.
// Adapt them before napi's JSON boundary, without serializing through toJSON.
function boundary(value, ancestors = new Set()) {
  if (value === undefined) return null
  if (value === null || typeof value !== 'object') return value
  if (ancestors.has(value)) throw new Error('Cyclic binding input')
  ancestors.add(value)
  try {
    if (Array.isArray(value) || value instanceof Set) {
      return Array.from(value, (item) => boundary(item, ancestors))
    }
    const result = Object.create(null)
    if (value instanceof Map) {
      for (const [key, item] of value) {
        if (typeof key !== 'string')
          throw new Error('Binding map keys must be strings')
        result[key] = boundary(item, ancestors)
      }
    } else {
      for (const key of Object.keys(value)) {
        result[key] = boundary(value[key], ancestors)
      }
    }
    return result
  } finally {
    ancestors.delete(value)
  }
}

exports.Output = native.Output
if (Symbol.dispose)
  native.Output.prototype[Symbol.dispose] = native.Output.prototype.free

exports.codeExtract = function codeExtract(
  filename,
  code,
  pkg,
  cssDir,
  singleCss,
  importMainCssInCode,
  importMainCssInCss,
  importAliases,
) {
  return native.codeExtract(
    filename,
    code,
    pkg,
    cssDir,
    !!singleCss,
    !!importMainCssInCode,
    !!importMainCssInCss,
    boundary(importAliases),
  )
}
exports.codeExtractWithoutSourceMap = function codeExtractWithoutSourceMap(
  filename,
  code,
  pkg,
  cssDir,
  singleCss,
  importMainCssInCode,
  importMainCssInCss,
  importAliases,
) {
  return native.codeExtractWithoutSourceMap(
    filename,
    code,
    pkg,
    cssDir,
    !!singleCss,
    !!importMainCssInCode,
    !!importMainCssInCss,
    boundary(importAliases),
  )
}
exports.reserveClassSlots = function reserveClassSlots(
  rows,
  pkg,
  singleCss,
  importMainCssInCode,
  importMainCssInCss,
  importAliases,
) {
  return native.reserveClassSlots(
    boundary(rows),
    pkg,
    !!singleCss,
    !!importMainCssInCode,
    !!importMainCssInCss,
    boundary(importAliases),
  )
}
exports.setModuleResolver = function setModuleResolver(resolver) {
  if (resolver == null) return native.setModuleResolver(null)
  if (typeof resolver !== 'function')
    throw new Error('Module resolver must be a function')
  native.setModuleResolver((specifier, importer) => {
    try {
      const module = resolver.call(null, specifier, importer)
      const path = module.path
      if (typeof path !== 'string') return null
      const code = module.code
      return typeof code === 'string' ? { path, code } : null
    } catch {
      // WASM call2/Reflect errors are resolver misses, not extraction exceptions.
      return null
    }
  })
}
exports.importSheet = (value) => native.importSheet(boundary(value))
exports.importClassMap = (value) => native.importClassMap(boundary(value))
exports.importFileMap = (value) => native.importFileMap(boundary(value))
exports.importCanonicalMap = (value) =>
  native.importCanonicalMap(boundary(value))
exports.importFileRoutes = (value) => native.importFileRoutes(boundary(value))
exports.registerTheme = (value) => native.registerTheme(boundary(value))
exports.registerShorthands = (value) =>
  native.registerShorthands(boundary(value))
exports.getCss = (fileNum, importMainCss) =>
  native.getCss(fileNum, !!importMainCss)
exports.setDebug = (debug) => native.setDebug(!!debug)
exports.isDebug = native.isDebug
exports.setPrefix = native.setPrefix
exports.getPrefix = native.getPrefix
exports.setAtomHoist = native.setAtomHoist
exports.seedFileMap = native.seedFileMap
exports.resetBuildState = native.resetBuildState
exports.getUnreservedClassSlotCount = native.getUnreservedClassSlotCount
exports.exportSheet = native.exportSheet
exports.exportClassMap = native.exportClassMap
exports.exportFileMap = native.exportFileMap
exports.exportCanonicalMap = native.exportCanonicalMap
exports.getDefaultTheme = native.getDefaultTheme
exports.getThemeInterface = native.getThemeInterface
exports.hasDevupUI = native.hasDevupUI
