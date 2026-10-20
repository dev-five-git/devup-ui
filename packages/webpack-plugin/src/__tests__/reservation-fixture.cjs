const { mkdirSync, rmSync, writeFileSync } = require('node:fs')
const { dirname, join } = require('node:path')

module.exports = function fixtureSources(fixture, mode) {
  function put(path, text) {
    const absolute = join(fixture, path)
    mkdirSync(dirname(absolute), { recursive: true })
    writeFileSync(absolute, text)
  }
  function rawSources(letter) {
    put(
      `physical/${letter}.css.ts`,
      `import { createVar, createContainer, layer } from '@vanilla-extract/css';
export const variable = createVar(); export const container = createContainer(); export const level = layer();`,
    )
  }
  function sources(letter) {
    const color = letter === 'A' ? 'red' : 'blue'
    if (mode !== 'watch') rawSources(letter)
    put(
      `physical/${letter}.js`,
      `import { css, keyframes, globalCss } from '@devup-ui/react';
import { variable, container, level } from './${letter}.css.ts';
const animation = keyframes({ from: { opacity: ${letter === 'A' ? 0 : 0.5} }, to: { opacity: 1 } });
globalCss({ '.${letter}-global': { color: '${color}' } });
export const ${letter} = [css({ color: '${color}', animationName: animation }), variable, container, level];`,
    )
  }
  if (mode === 'watch') for (const letter of ['A', 'B']) rawSources(letter)
  put('devup.json', '{}')
  put('src/plain.js', 'export const plain = 1;')
  const physicalImports = `export { A } from '../physical/A.js';
export { B } from '../physical/B.js';
export { variable as a } from '../physical/A.css.ts';
export { variable as b } from '../physical/B.css.ts';`
  put('src/reservation-inputs.js', physicalImports)
  put(
    'src/entry.js',
    `import { css } from '@devup-ui/react';
import { child } from './child.js'; import { dist } from '@slot/ui';
export const entry = [css({ display: 'grid' }), child, dist];`,
  )
  put(
    'src/child.js',
    `import { css } from '@devup-ui/react'; export const child = css({ padding: '12px' });`,
  )
  put(
    'src/Unused.js',
    `import { globalCss } from '@devup-ui/react'; globalCss({ '.unused': { color: 'magenta' } });`,
  )
  put(
    'src/Invalid.js',
    `import { css } from '@devup-ui/react'; export const invalid = css({ color: props.unknown });`,
  )
  put(
    'node_modules/@slot/ui/package.json',
    JSON.stringify({ name: '@slot/ui', main: 'dist/index.js' }),
  )
  put(
    'node_modules/@slot/ui/dist/index.js',
    `import { css } from '@devup-ui/react'; export const dist = css({ borderColor: 'orange' });`,
  )
  if (mode === 'atom') {
    for (const route of ['a', 'b'])
      put(
        `src/app/${route}/page.js`,
        `import { child } from '../../child.js'; export const page = child;`,
      )
  }
  if (mode === 'noop') {
    rmSync(join(fixture, 'src'), { recursive: true })
    put(
      'src/entry.js',
      `import { css } from '@devup-ui/react'; export const entry = css({ display: 'grid' });
import './0-Generated.js';
${physicalImports}`,
    )
  }
  return { put, sources }
}
