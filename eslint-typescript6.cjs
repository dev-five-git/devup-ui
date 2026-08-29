// TypeScript 7 is a native CLI and intentionally has no JavaScript compiler
// API yet. ESLint's TypeScript parser still requires that API, so route only
// its `typescript` imports to the official side-by-side TypeScript 6 package.
const Module = require('node:module')
const path = require('node:path')
const { pathToFileURL } = require('node:url')

const originalResolveFilename = Module._resolveFilename
const typescript6Package =
  require.resolve('@typescript/typescript6/package.json')
const typescript6Directory = path.dirname(typescript6Package)

function resolveTypeScript6(request) {
  if (request === 'typescript') {
    return path.join(typescript6Directory, 'lib', 'typescript.js')
  }
  if (request.startsWith('typescript/')) {
    return path.join(typescript6Directory, request.slice('typescript/'.length))
  }
}

Module.registerHooks({
  resolve(specifier, context, nextResolve) {
    const resolved = resolveTypeScript6(specifier)
    return resolved
      ? { shortCircuit: true, url: pathToFileURL(resolved).href }
      : nextResolve(specifier, context)
  },
})

Module._resolveFilename = function resolveFilename(request, parent, ...rest) {
  return (
    resolveTypeScript6(request) ??
    originalResolveFilename.call(this, request, parent, ...rest)
  )
}
