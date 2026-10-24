const assert = require('node:assert/strict')
const { readFileSync } = require('node:fs')
const { Module, createRequire } = require('node:module')
const { dirname } = require('node:path')
const { compileFunction, constants } = require('node:vm')

/** @typedef {typeof import('../html-reservation').htmlReservationEntries} Parser */

/** Rebind only the private lexical parser in an in-memory copy of the emitted CJS.
 * The published file and its exports remain untouched.
 * @param {(parse: Parser) => Parser} wrap
 * @returns {() => void} */
function rebindParser(wrap) {
  const filename = require.resolve('../../dist/' + 'index.cjs')
  assert.equal(require.cache[filename], undefined)
  const code = readFileSync(filename, 'utf8')
  const parsers = [
    ...code.matchAll(
      /async function ([\w$]+)\([^)]*\)\s*\{\s*(?:const|let)\s*\{[^}]*\}\s*=\s*await import\(["']parse5["']\)/g,
    ),
  ]
  assert.equal(parsers.length, 1, 'one emitted asynchronous HTML parser')
  const parser = parsers[0]?.[1]
  assert(parser)
  const evaluate = compileFunction(
    code +
      `
return (wrap) => {
  const parse = ${parser};
  ${parser} = wrap(parse);
  return () => { ${parser} = parse; };
};`,
    ['exports', 'require', 'module', '__filename', '__dirname'],
    {
      filename,
      importModuleDynamically: constants.USE_MAIN_CONTEXT_DEFAULT_LOADER,
    },
  )
  const loaded = new Module(filename)
  loaded.filename = filename
  /** @type {(wrap: (parse: Parser) => Parser) => () => void} */
  const bind = evaluate.call(
    loaded.exports,
    loaded.exports,
    createRequire(filename),
    loaded,
    filename,
    dirname(filename),
  )
  const restore = bind(wrap)
  loaded.loaded = true
  require.cache[filename] = loaded
  return restore
}

module.exports = { rebindParser }
