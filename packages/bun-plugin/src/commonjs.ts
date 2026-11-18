import { randomUUID } from 'node:crypto'
import Module, { createRequire } from 'node:module'
import { dirname } from 'node:path'
import { constants, runInThisContext } from 'node:vm'

/**
 * CommonJS under the Bun runtime.
 *
 * Bun reads the contents an `onLoad` callback returns as an ES module. When
 * those contents are CommonJS, Bun wraps them in the CommonJS function wrapper
 * but never calls it: the module body does not run, and `require()` and
 * `import` see `{}` (Bun 1.4). `Bun.build` is not affected.
 *
 * So the runtime plugin runs a CommonJS module itself and hands Bun an ES
 * module that exports the result the way Bun exports CommonJS: `require()`
 * returns `module.exports` through the `"module.exports"` export, and `import`
 * gets `default` and the named exports. As in Bun's own loader, the module
 * runs when it is loaded, before the ES modules that import it.
 */

type ModuleFunction = (this: unknown, ...args: unknown[]) => void

/** Words CommonJS uses; a file without any is an ES module or a script */
const commonJsWords = /\b(?:module|exports)\b|\bexport\s*=/
/** Extensions Node runs as CommonJS unless they hold module syntax */
const commonJsExtension = /\.c[jt]s$/
const esModuleExtension = /\.m[jt]s$/

// Bun's transpiler replaces `__filename` and `__dirname` with constants for a
// file it has no path of, and inlines `process.env.NODE_ENV`. The module reads
// them when it runs instead.
const filenameAlias = '__devupUiFilename'
const dirnameAlias = '__devupUiDirname'
const typeScript = new Bun.Transpiler({
  loader: 'ts',
  target: 'bun',
  define: {
    __filename: filenameAlias,
    __dirname: dirnameAlias,
    'process.env.NODE_ENV': 'process.env.NODE_ENV',
  },
})
const javaScript = new Bun.Transpiler({ loader: 'js', target: 'bun' })

/** The values the generated ES modules export, by load */
const loaded = new Map<number, unknown>()
// Unique to this copy of the plugin, so a second copy keeps its own values
const loadedKey = `devup-ui:commonjs:${randomUUID()}`
Object.defineProperty(globalThis, Symbol.for(loadedKey), { value: loaded })
let loads = 0

/**
 * `code` as a CommonJS script, or `undefined` for an ES module. Bun reports
 * the syntax error of code that does not parse when it loads the source.
 */
function toScript(code: string, loader: 'ts' | 'js') {
  try {
    const script = loader === 'ts' ? typeScript.transformSync(code) : code
    const { exports, imports } = javaScript.scan(script)
    return exports.length > 0 ||
      imports.some(({ kind }) => kind === 'import-statement')
      ? undefined
      : script
  } catch {
    return undefined
  }
}

/** The module function of `script`, or `undefined` when it is no script */
function compile(script: string, path: string): ModuleFunction | undefined {
  try {
    return runInThisContext(
      `(function (exports, require, module, __filename, __dirname, ${filenameAlias}, ${dirnameAlias}) {${
        // A hashbang is only valid at the start of a file
        script.replace(/^#!/, '//')
      }\n})`,
      {
        filename: path,
        importModuleDynamically: constants.USE_MAIN_CONTEXT_DEFAULT_LOADER,
      },
    )
  } catch {
    // Syntax only an ES module holds, such as top-level await or
    // `import.meta`: Bun loads the source as an ES module
    return undefined
  }
}

/** Runs a CommonJS module and returns its `module.exports` */
function evaluate(run: ModuleFunction, path: string): unknown {
  const require = createRequire(path)
  // `require()` registers the module before Bun calls `onLoad`; `import`
  // does not, so the module is registered here. A cycle back to this file
  // then gets the exports so far, as it does in Node.
  let module = require.cache[path]
  if (!module || module.loaded) {
    module = new Module(path)
    module.filename = path
    require.cache[path] = module
  }
  const directory = dirname(path)
  try {
    run.call(
      module.exports,
      module.exports,
      require,
      module,
      path,
      directory,
      path,
      directory,
    )
  } catch (error) {
    delete require.cache[path]
    throw error
  }
  module.loaded = true
  return module.exports
}

/** An ES module exporting `value` as Bun exports CommonJS */
function exportingSource(value: unknown) {
  const id = loads++
  loaded.set(id, value)
  const names =
    (typeof value === 'object' && value !== null) || typeof value === 'function'
      ? Object.keys(value).filter(
          (name) => name !== 'default' && name !== 'module.exports',
        )
      : []
  return [
    `const loaded = globalThis[Symbol.for(${JSON.stringify(loadedKey)})]`,
    `const value = loaded.get(${id})`,
    `loaded.delete(${id})`,
    // Babel and TypeScript mark an ES module compiled to CommonJS
    'const main = value?.__esModule ? value.default : value',
    'export { value as "module.exports", main as default }',
    ...names.map(
      (name, index) =>
        `const name${index} = value[${JSON.stringify(name)}]\nexport { name${index} as ${JSON.stringify(name)} }`,
    ),
  ].join('\n')
}

/**
 * Runs `code` when it is a CommonJS module, and returns the ES module Bun is
 * to load in its place; `undefined` for an ES module.
 *
 * `stylesheet` is the stylesheet import the extractor added. Under the runtime
 * it loads nothing, and CommonJS cannot hold an import.
 */
export function commonJsSource(
  path: string,
  code: string,
  loader: string,
  stylesheet?: string,
) {
  if (
    (loader !== 'ts' && loader !== 'js') ||
    esModuleExtension.test(path) ||
    !(commonJsExtension.test(path) || commonJsWords.test(code))
  )
    return undefined
  const script = toScript(
    stylesheet === undefined
      ? code
      : code.replace(`import ${JSON.stringify(stylesheet)};\n`, ''),
    loader,
  )
  const run = script === undefined ? undefined : compile(script, path)
  return run && exportingSource(evaluate(run, path))
}
