'use strict'
const { withAccess } = require('./interop.cjs')
const encoder = new TextEncoder()
const decoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true })
function string(value) {
  if (typeof value === 'string') return value
  const length = value.length
  void (+length | 0)
  const byte = new Uint8Array(1)
  let offset = 0,
    text = ''
  for (; offset < length; offset++) {
    const code = value.charCodeAt(offset)
    if (code > 127) break
    byte[0] = code
    text += String.fromCharCode(byte[0])
  }
  if (offset !== length) {
    const tail = offset === 0 ? value : value.slice(offset)
    const view = new Uint8Array(tail.length * 3)
    const { written } = encoder.encodeInto(tail, view)
    text += decoder.decode(view.subarray(0, written))
  }
  return text
}
exports.createBinding = function createBinding(raw) {
  raw.init()
  const call = (fn, value) => withAccess((access) => fn(value, access))
  const boolean = (value) => (+value | 0) !== 0
  const index = (value) => (value == null ? undefined : value >>> 0)
  function codeExtract(
    filename,
    code,
    packageName,
    cssDir,
    singleCss,
    importCode,
    importCss,
    aliases,
  ) {
    return call(raw.extract, [
      string(filename),
      string(code),
      string(packageName),
      string(cssDir),
      boolean(singleCss),
      boolean(importCode),
      boolean(importCss),
      aliases,
      true,
    ])
  }
  function codeExtractWithoutSourceMap(
    filename,
    code,
    packageName,
    cssDir,
    singleCss,
    importCode,
    importCss,
    aliases,
  ) {
    return call(raw.extract, [
      string(filename),
      string(code),
      string(packageName),
      string(cssDir),
      boolean(singleCss),
      boolean(importCode),
      boolean(importCss),
      aliases,
      false,
    ])
  }
  function setModuleResolver(resolver) {
    raw.setResolver(
      resolver == null
        ? undefined
        : (specifier, importer) => {
            try {
              const result = resolver.call(null, specifier, importer)
              const path = Reflect.get(result, 'path')
              if (typeof path !== 'string') return undefined
              const code = Reflect.get(result, 'code')
              return typeof code === 'string' ? { path, code } : undefined
            } catch {
              return undefined
            }
          },
    )
  }
  function seedFileMap(files) {
    void (files.length * 4)
    const copied = []
    for (let i = 0; i < files.length; i++) copied.push(files[i])
    copied.length = +files.length >>> 0
    for (const value of copied)
      if (typeof value !== 'string')
        throw new Error('array contains a value of the wrong type')
    raw.seedFileMap(copied)
  }
  if (Symbol.dispose)
    raw.Output.prototype[Symbol.dispose] = raw.Output.prototype.free
  return {
    Output: raw.Output,
    codeExtract,
    codeExtractWithoutSourceMap,
    exportCanonicalMap: raw.exportCanonicalMap,
    exportClassMap: raw.exportClassMap,
    exportFileMap: raw.exportFileMap,
    exportSheet: raw.exportSheet,
    getCss: (file, main) => raw.getCss(index(file), boolean(main)),
    getDefaultTheme: raw.getDefaultTheme,
    getPrefix: raw.getPrefix,
    getThemeInterface: (pkg, color, typography, length, shadows, theme) =>
      raw.getThemeInterface(string(pkg), [
        string(color),
        string(typography),
        string(length),
        string(shadows),
        string(theme),
      ]),
    getUnreservedClassSlotCount: raw.getUnreservedClassSlotCount,
    hasDevupUI: (file, code, pkg) =>
      raw.hasDevupUi(string(file), string(code), string(pkg)),
    importCanonicalMap: (value) => call(raw.importCanonicalMap, value),
    importClassMap: (value) => call(raw.importClassMap, value),
    importFileMap: (value) => call(raw.importFileMap, value),
    importFileRoutes: (value) => call(raw.importFileRoutes, value),
    importSheet: (value) => call(raw.importSheet, value),
    isDebug: raw.isDebug,
    registerShorthands: (value) => call(raw.registerShorthands, value),
    registerTheme: (value) => call(raw.registerTheme, value),
    reserveClassSlots: (rows, pkg, single, code, css, aliases) =>
      call(raw.reserve, [
        rows,
        string(pkg),
        boolean(single),
        boolean(code),
        boolean(css),
        aliases,
      ]),
    resetBuildState: () => raw.reset(false),
    resetStateForTesting: () => raw.reset(true),
    seedFileMap,
    setAtomHoist: (value) => raw.setAtomHoist(index(value)),
    setDebug: (value) => raw.setDebug(boolean(value)),
    setModuleResolver,
    setPrefix: (value) => raw.setPrefix(value == null ? undefined : value),
  }
}
