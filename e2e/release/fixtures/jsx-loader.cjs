const ts = require('@typescript/typescript6')

module.exports = function transformJsx(source) {
  return ts.transpileModule(source, {
    compilerOptions: {
      jsx: ts.JsxEmit.ReactJSX,
      target: ts.ScriptTarget.ESNext,
      module: ts.ModuleKind.ESNext,
    },
    fileName: this.resourcePath,
  }).outputText
}
