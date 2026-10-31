import assert from 'node:assert/strict'

import { box, pkg } from './corpus.mjs'

export const code = box('color="orange" p={7}')
const row = ['src/new.tsx', code, 'df']
const extraction = () => [
  'src/new.tsx',
  code,
  pkg,
  'df',
  false,
  false,
  false,
  {},
]
const extractSlots =
  'filename:s code:s _package:s css_dir:s single_css:b import_main_css_in_code:b import_main_css_in_css:b import_aliases:aliases'
// Independent acceptance oracle: no product validator or engine diagnostics imported.
export const positions = [
  ['codeExtract', extractSlots, extraction],
  ['codeExtractWithoutSourceMap', extractSlots, extraction],
  ['getCss', 'file_num:n import_main_css:b', () => [null, false]],
  [
    'getThemeInterface',
    'package_name:s color_interface_name:s typography_interface_name:s length_interface_name:s shadows_interface_name:s theme_interface_name:s',
    () => [pkg, 'Colors', 'Typography', 'Length', 'Shadow', 'Theme'],
  ],
  [
    'hasDevupUI',
    'filename:s code:s _package:s',
    () => [...extraction().slice(0, 3), { '@emotion/styled': null }],
  ],
  ['importCanonicalMap', 'map_object:canonical', () => [{}]],
  ['importClassMap', 'sheet_object:class', () => [{}]],
  ['importFileMap', 'sheet_object:filemap', () => [{}]],
  ['importFileRoutes', 'map_object:routes', () => [{}]],
  ['importSheet', 'sheet_object:sheet', () => [{}]],
  ['registerShorthands', 'shorthands:shorthands', () => [{}]],
  ['registerTheme', 'theme_object:theme', () => [{}]],
  [
    'reserveClassSlots',
    'rows:rows _package:s single_css:b import_main_css_in_code:b _import_main_css_in_css:b import_aliases:aliases',
    () => [[row], pkg, false, false, false, {}],
  ],
  ['seedFileMap', 'files:seed', () => [['src/new.tsx']]],
  ['setAtomHoist', 'threshold:n', () => [2]],
  ['setDebug', 'debug:b', () => [true]],
  ['setModuleResolver', 'resolver:resolver', () => [null]],
  ['setPrefix', 'prefix:prefix', () => ['valid-']],
].flatMap(([fn, slots, args]) =>
  slots.split(' ').map((slot, index) => {
    const [parameter, kind] = slot.split(':')
    return { fn, parameter, kind, index, args }
  }),
)
const labels = {
  s: 'a string',
  b: 'a boolean',
  n: 'a number, null or undefined',
  prefix: 'a string, null or undefined',
  resolver: 'a function, null or undefined',
  seed: 'an array of strings',
  aliases: 'an alias mapping',
  canonical: 'a canonical mapping',
  class: 'a nested class-index mapping',
  filemap: 'a file-index mapping',
  routes: 'a file-to-route mapping',
  sheet: 'a sheet state',
  shorthands: 'a shorthand mapping',
  theme: 'a theme',
  rows: 'an iterable of 3-string rows',
}
const roots = [7, false, 'bad', null, undefined]
const property = (s) => ({
  properties: { file: { 0: { 0: [{ c: 'a', p: 'color', v: 'red', s }] } } },
})
const invalid = {
  s: [false, {}, 7, undefined, null],
  b: [7, 'false', null, undefined],
  n: ['2', false, 1n, Symbol('number'), {}],
  prefix: [false, true, 7, {}],
  resolver: [false, true, 7, 'callback', {}],
  seed: [
    ...roots,
    {},
    new Set(['src/new.tsx']),
    ['src/new.tsx', 7],
    Object.assign(['src/new.tsx'], { length: 2 }),
  ],
  aliases: [...roots, { bad: 7 }, { bad: false }, { bad: {} }, new Set([null])],
  canonical: [...roots, { file: 7 }, { file: null }, new Set([null])],
  class: [
    ...roots,
    { file: 7 },
    { file: { color: 'bad' } },
    { file: null },
    new Set([null]),
  ],
  filemap: [...roots, { file: 'bad' }, { file: {} }, new Set([null])],
  routes: [
    ...roots,
    { file: 7 },
    { file: [0, 'bad'] },
    { file: [null] },
    new Set([null]),
  ],
  sheet: [
    ...roots,
    { css: 7 },
    { css: { file: 7 } },
    { css: { file: [{ css: 7 }] } },
    property({ Unknown: [] }),
    property({ Global: ['body'] }),
    property({ Selector: 7 }),
    { keyframes: { file: { spin: { from: [['color']] } } } },
  ],
  theme: [
    ...roots,
    { colors: 7 },
    { colors: { default: 7 } },
    { colors: { default: { primary: true } } },
    { typography: { heading: { fontSize: {} } } },
    { length: { default: { gutter: {} } } },
    { shadows: { default: { card: {} } } },
  ],
  shorthands: [
    ...roots,
    { insetX: 7 },
    { insetX: ['left', 7] },
    { insetX: null },
    new Set([null]),
  ],
  rows: [
    ...roots,
    {},
    [row, null],
    [row, 7],
    [row, ['src/bad.tsx', 7, 'df']],
    [row, ['src/bad.tsx', code]],
    [row, ['src/bad.tsx', code, 'df', 'extra']],
    [row, ['src/bad.tsx', code, {}]],
  ],
}
export const cases = positions.flatMap((position) =>
  invalid[position.kind].map((value, variant) => ({
    ...position,
    id: `${position.fn}.${position.parameter}.${variant}`,
    expected: `${position.fn}(): argument ${position.parameter} must be ${labels[position.kind]}`,
    input: () =>
      position
        .args()
        .map((good, index) => (index === position.index ? value : good)),
  })),
)
assert.equal(positions.length, 45)
assert.equal(new Set(positions.map((p) => p.fn)).size, 18)
assert.equal(new Set(cases.map((c) => c.id)).size, cases.length)
