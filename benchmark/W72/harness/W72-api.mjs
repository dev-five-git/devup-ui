import assert from 'node:assert/strict'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import {
  aliases,
  extract,
  hash,
  load,
  out,
  output,
  reset,
} from './W72-corpus.mjs'

const variant = process.argv[2]
const binding = await load(variant)
const observations = []
const source = {
  filename: 'W72-api.tsx',
  code: "import {Box,css,keyframes} from '@devup-ui/react';const k=keyframes({from:{opacity:0},to:{opacity:1}});export const x=<Box color='$primary' insetX={2} animation={`${k} 1s`} _hover={{p:[1,2]}}/>;",
}
reset(binding)
binding.setPrefix('w72-')
binding.setDebug(true)
assert.equal(binding.getPrefix(), 'w72-')
assert.equal(binding.isDebug(), true)
binding.setDebug(false)
const apiTheme = {
  colors: {
    default: { primary: 'red', z: 'blue' },
    dark: { primary: 'blue', z: 'red' },
  },
  length: { default: { space: ['2px', null, '8px'] } },
}
binding.registerTheme(apiTheme)
binding.registerShorthands(new Map([['insetX', ['left', 'right']]]))
assert.equal(
  binding.hasDevupUI(source.filename, source.code, '@devup-ui/react'),
  true,
)
assert.equal(binding.hasDevupUI('x.ts', 'const x=1', '@devup-ui/react'), false)
observations.push(output(extract(binding, source)))
observations.push(
  binding.getDefaultTheme(),
  binding.getThemeInterface('@devup-ui/react', 'C', 'T', 'L', 'S', 'Theme'),
)
const before = binding.getCss(null, false)
const saved = [
  binding.exportSheet(),
  binding.exportClassMap(),
  binding.exportFileMap(),
  binding.exportCanonicalMap(),
]
binding.importSheet(JSON.parse(saved[0]))
binding.importClassMap(JSON.parse(saved[1]))
binding.importFileMap(JSON.parse(saved[2]))
binding.importCanonicalMap(JSON.parse(saved[3]))
binding.registerTheme(apiTheme)
assert.equal(binding.getCss(null, false), before)
observations.push(before)
reset(binding)
assert.equal(binding.getPrefix(), undefined)
binding.seedFileMap(['b.tsx', 'a.tsx'])
binding.importCanonicalMap({ 'b.tsx': 'a.tsx' })
binding.importFileRoutes({ 'a.tsx': [0, 1], 'b.tsx': new Set([0, 1]) })
binding.setAtomHoist(2)
const atom = {
  filename: 'a.tsx',
  code: "import {css,keyframes} from '@devup-ui/react';const k=keyframes({to:{opacity:0}});export const a=css({color:'red',animation:`${k} 1s`});",
}
observations.push(
  output(extract(binding, atom, 'map', false)),
  binding.getCss(null, false),
)
reset(binding)
let calls = 0
binding.setModuleResolver((specifier, importer) => {
  assert.equal(specifier, './tokens')
  assert.equal(importer, 'W72-resolver.tsx')
  calls++
  return { path: 'tokens.ts', code: "export const color='orchid';" }
})
const callback = output(
  extract(binding, {
    filename: 'W72-resolver.tsx',
    code: "import {css} from '@devup-ui/react';import {color} from './tokens';export const a=css({color});",
  }),
)
assert.ok(calls > 0)
assert.deepEqual(callback.dependencies, ['tokens.ts'])
assert.ok(callback.css.includes('orchid'))
observations.push(callback)
binding.setModuleResolver(() => {
  throw new Error('expected resolver miss')
})
assert.throws(() =>
  extract(binding, {
    filename: 'W72-missing.tsx',
    code: "import {css} from '@devup-ui/react';import {color} from './missing';export const a=css({color});",
  }),
)
reset(binding)
const rows = [
  [
    'b.tsx',
    "import {Box} from '@devup-ui/react';export const b=<Box bg='blue'/>;",
    'W72-css',
  ],
  [
    'a.tsx',
    "import {Box} from '@devup-ui/react';export const a=<Box bg='red'/>;",
    'W72-css',
  ],
]
binding.reserveClassSlots(rows, '@devup-ui/react', true, false, false, aliases)
for (const [filename, code] of rows.toReversed())
  observations.push(output(extract(binding, { filename, code })))
assert.equal(binding.getUnreservedClassSlotCount(), 0)
const prior = binding.exportClassMap()
assert.throws(() =>
  binding.reserveClassSlots(
    [['broken']],
    '@devup-ui/react',
    true,
    false,
    false,
    aliases,
  ),
)
assert.equal(binding.exportClassMap(), prior)
observations.push(binding.exportClassMap())
reset(binding)
const properties = [
  'color',
  'backgroundColor',
  'borderColor',
  'outlineColor',
  'width',
  'height',
  'minWidth',
  'minHeight',
  'maxWidth',
  'maxHeight',
  'top',
  'right',
  'bottom',
  'left',
  'fontSize',
  'lineHeight',
  'letterSpacing',
  'wordSpacing',
  'textIndent',
  'borderWidth',
  'borderRadius',
  'outlineWidth',
  'outlineOffset',
  'columnGap',
  'rowGap',
  'flexBasis',
  'marginTop',
  'marginRight',
  'marginBottom',
  'marginLeft',
  'paddingTop',
  'paddingRight',
]
const leaves = properties
  .map((p, i) => `${p}:(trace.push('leaf:${i}'),previous+='x')`)
  .join(',')
const defaults = properties.map((p) => `${p}:'initial'`).join(',')
for (const filename of ['W72-captures.ts', 'W72-captures.cts']) {
  const code = `import {styled} from '@devup-ui/react';const trace=[];let previous='';let outer=true,inner=true;const targets={get div(){return 'article'}};const name='div';const Card=styled(targets[name]).attrs({id:'card'})({${defaults},...(outer?{...(inner?{${leaves}}:{})}:{})});`
  observations.push(output(extract(binding, { filename, code })))
}
writeFileSync(
  join(out, `W72-api-${variant}.json`),
  JSON.stringify(observations),
)
process.stdout.write(
  JSON.stringify({
    variant,
    checks: 'passed',
    observations: observations.length,
    sha256: hash(JSON.stringify(observations)),
  }) + '\n',
)
