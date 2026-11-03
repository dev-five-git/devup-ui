// TypeScript 7 is a native CLI and intentionally has no JavaScript compiler
// API yet. ESLint's TypeScript parser still requires that API, so route only
// its `typescript` imports to the official side-by-side TypeScript 6 package.
const Module = require('node:module')
const { pathToFileURL } = require('node:url')

const originalResolveFilename = Module._resolveFilename

function resolveTypeScript6(request) {
  if (request === 'typescript' || request.startsWith('typescript/')) {
    return require.resolve(
      '@typescript/typescript6' + request.slice('typescript'.length),
    )
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
