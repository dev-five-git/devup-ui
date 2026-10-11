import assert from 'node:assert/strict'

export function verifyIgnoredSemantics(wasm, method, resolver) {
  const extract = (filename, code) => {
    wasm.resetBuildState()
    wasm.setModuleResolver(resolver)
    const output = wasm[method](
      filename,
      code,
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    const result = {
      code: output.code,
      css: wasm.getCss(undefined, false),
      dependencies: output.dependencies,
    }
    output.free()
    return result
  }
  for (const key of [
    'constructor',
    'toString',
    '__proto__',
    'hasOwnProperty',
  ]) {
    const named = extract(
      '/src/named.css.ts',
      `import {style} from '@devup-ui/react'; import {${key} as value} from 'empty'; export const cls = style({color: typeof value === 'undefined' ? 'red' : 'blue'});`,
    )
    assert.match(named.css, /color:red/, `${method}: named ${key}`)
    assert.doesNotMatch(named.css, /color:blue/)
    assert.deepEqual(named.dependencies, [])
  }
  for (const binding of [
    "import {missing as value} from 'empty';",
    "import def from 'empty'; const value = def.missing;",
    "import * as ns from 'empty'; const value = ns.missing;",
    "import {value} from 'barrel';",
    "import def from 'whole'; const value = def.missing;",
    'const record = {}; const value = record.missing;',
  ]) {
    const shadowed = extract(
      '/src/shadowed.tsx',
      `import {Box} from '@devup-ui/react'; ${binding} export const View = (undefined, read) => <Box color={read(value)}/>;`,
    )
    const argument = /read\(([^)]+)\)/.exec(shadowed.code)?.[1]
    assert(argument, shadowed.code)
    const selected = new Function(
      'undefined',
      'read',
      'value',
      `return read(${argument});`,
    )('blue', (value) => value ?? 'red', undefined)
    assert.equal(selected, 'red', `${method}: ${binding}`)
  }
  for (const binding of [
    "import def from 'real';",
    'const def = {};',
    "import def from 'empty';",
    "import * as def from 'empty';",
    "import def from 'whole';",
  ]) {
    for (const [key, expected] of [
      ['constructor', 'Object'],
      ['toString', 'Object.prototype.toString'],
    ]) {
      const real = extract(
        '/src/real.tsx',
        `import {Box} from '@devup-ui/react'; ${binding} export const view = <Box color={def.${key} === ${expected} ? 'red' : 'blue'}/>;`,
      )
      const condition = /className=\{([^?]+)\?/.exec(real.code)?.[1]
      assert(condition, real.code)
      assert.equal(
        new Function('def', `return ${condition};`)({}),
        true,
        `${method}: ${binding} ${key}`,
      )
    }
  }
  for (const binding of [
    "import def from 'empty';",
    "import * as def from 'empty';",
  ]) {
    const mutated = extract(
      '/src/mutated.tsx',
      `import {Box} from '@devup-ui/react'; ${binding} def.color = 'blue'; export const view = <Box color={def.color}/>;`,
    )
    assert.match(mutated.code, /def\.color/)
    assert.match(mutated.css, /color:var\(/)
    assert.deepEqual(mutated.dependencies, [])
  }
}
