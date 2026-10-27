'use strict'
const { boundary, withAccess } = require('./interop.cjs')
exports.createBinding = function createBinding(raw) {
  raw.init()
  const call = (fn, value) => withAccess((access) => fn(value, access))
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
      filename,
      code,
      packageName,
      cssDir,
      singleCss,
      importCode,
      importCss,
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
      filename,
      code,
      packageName,
      cssDir,
      singleCss,
      importCode,
      importCss,
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
    raw.seedFileMap(files)
  }
  if (Symbol.dispose)
    raw.Output.prototype[Symbol.dispose] = raw.Output.prototype.free
  return boundary.wrap({
    Output: raw.Output,
    codeExtract,
    codeExtractWithoutSourceMap,
    exportCanonicalMap: raw.exportCanonicalMap,
    exportClassMap: raw.exportClassMap,
    exportFileMap: raw.exportFileMap,
    exportSheet: raw.exportSheet,
    getCss: (file, main) => raw.getCss(index(file), main),
    getDefaultTheme: raw.getDefaultTheme,
    getPrefix: raw.getPrefix,
    getThemeInterface: (pkg, color, typography, length, shadows, theme) =>
      raw.getThemeInterface(pkg, [color, typography, length, shadows, theme]),
    getUnreservedClassSlotCount: raw.getUnreservedClassSlotCount,
    hasDevupUI: (file, code, pkg, aliases) =>
      call(raw.hasDevupUi, [file, code, pkg, aliases]),
    importCanonicalMap: (value) => call(raw.importCanonicalMap, value),
    importClassMap: (value) => call(raw.importClassMap, value),
    importFileMap: (value) => call(raw.importFileMap, value),
    importFileRoutes: (value) => call(raw.importFileRoutes, value),
    importSheet: (value) => call(raw.importSheet, value),
    isDebug: raw.isDebug,
    registerShorthands: (value) => call(raw.registerShorthands, value),
    registerTheme: (value) => call(raw.registerTheme, value),
    reserveClassSlots: (rows, pkg, single, code, css, aliases) =>
      call(raw.reserve, [rows, pkg, single, code, css, aliases]),
    resetBuildState: () => raw.reset(false),
    resetStateForTesting: () => raw.reset(true),
    seedFileMap,
    setAtomHoist: (value) => raw.setAtomHoist(index(value)),
    setDebug: (value) => raw.setDebug(value),
    setModuleResolver,
    setPrefix: (value) => raw.setPrefix(value == null ? undefined : value),
  })
}
