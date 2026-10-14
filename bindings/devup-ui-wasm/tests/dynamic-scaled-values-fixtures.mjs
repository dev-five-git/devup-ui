import { additionalFixtures } from './dynamic-scaled-values-additional-fixtures.mjs'

const components = [
  'Box',
  'Flex',
  'Grid',
  'VStack',
  'Center',
  'Text',
  'Image',
  'Input',
  'Button',
]
export const fixtures = []
const add = (
  name,
  dynamic,
  literal,
  value,
  annotation = '',
  props = ['paddingTop'],
  hover = false,
) => {
  fixtures.push({ name, dynamic, literal, value, annotation, props, hover })
}
for (const component of components) {
  add(
    `${component}-number`,
    `<${component} p={n}/>`,
    `<${component} p={4}/>`,
    '4',
    ':number',
  )
}
for (const [prop, computed] of Object.entries({
  p: 'paddingTop',
  m: 'marginTop',
  w: 'width',
  h: 'height',
  minW: 'minWidth',
  maxH: 'maxHeight',
  px: 'paddingLeft',
  py: 'paddingTop',
  mx: 'marginLeft',
  my: 'marginTop',
  gap: 'gap',
  rowGap: 'rowGap',
  columnGap: 'columnGap',
  borderRadius: 'borderTopLeftRadius',
  fontSize: 'fontSize',
  letterSpacing: 'letterSpacing',
  boxSize: 'width',
})) {
  add(
    `${prop}-number`,
    `<Box ${prop}={n}/>`,
    `<Box ${prop}={4}/>`,
    '4',
    ':number',
    [computed],
  )
}
for (const [value, annotation, label] of [
  ['4', '', 'unknown-number'],
  ['4', ':string|number', 'union-number'],
  ["'4'", ':string|number', 'union-string'],
  ["'4'", ':string', 'string-number'],
  ["'-2'", ':string', 'negative-string'],
  ["'0.5'", ':string', 'fraction-string'],
  ["' 4 '", ':string', 'spaced-string'],
  ["'10px'", ':string', 'length-string'],
]) {
  add(label, '<Box m={n}/>', `<Box m={${value}}/>`, value, annotation, [
    'marginTop',
  ])
}
add('width-100', '<Box w={n}/>', '<Box w={100}/>', '100', ':number', ['width'])
add('auto', '<Box w={n}/>', "<Box w='auto'/>", "'auto'", ":'auto'|'10px'", [
  'width',
])
add('template-unit', '<Box p={`${n}px`}/>', "<Box p='10px'/>", '10')
add('template-number', '<Box p={`1${n}2`}/>', "<Box p='12'/>", "''")
add('template-escape', '<Box p={`${n}\\x34`}/>', '<Box p={`\\x34`}/>', "''")
add('operator-number', '<Box p={n*2}/>', '<Box p={4}/>', '2')
add(
  'time-number',
  '<Box transitionDuration={n}/>',
  '<Box transitionDuration={200}/>',
  '200',
  ':number',
  ['transitionDuration'],
)
add(
  'time-string',
  '<Box transitionDuration={n}/>',
  "<Box transitionDuration='200'/>",
  "'200'",
  ':string',
  ['transitionDuration'],
)
add(
  'unitless-number',
  '<Box opacity={n}/>',
  '<Box opacity={0.5}/>',
  '0.5',
  ':number',
  ['opacity'],
)
add('percentage-string', '<Box w={n}/>', "<Box w='50%'/>", "'50%'", ':string', [
  'width',
])
add(
  'calc-string',
  '<Box p={n}/>',
  "<Box p='calc(2 * 4px)'/>",
  "'calc(2 * 4px)'",
  ':string',
)
add('responsive', "<Box p={[n,n*2,'12px']}/>", "<Box p={[4,8,'12px']}/>", '4')
add(
  'hover',
  '<Box p={1} _hover={{p:n}}/>',
  '<Box p={1} _hover={{p:4}}/>',
  '4',
  ':number',
  ['paddingTop'],
  true,
)
add(
  'responsive-hover',
  "<Box p={1} _hover={{p:[n,n*2,'12px']}}/>",
  "<Box p={1} _hover={{p:[4,8,'12px']}}/>",
  '4',
  '',
  ['paddingTop'],
  true,
)
add('jsx-runtime', 'jsx(Box,{p:n})', 'jsx(Box,{p:4})', '4', ':number')
add(
  'selector-array',
  '<Box p={1} _hover={[{p:n},{p:n*2}]}/>',
  '<Box p={1} _hover={[{p:4},{p:8}]}/>',
  '4',
  '',
  ['paddingTop'],
  true,
)
add(
  'selector-child',
  "<Box selectors={{'& > span':{p:n}}}><span/></Box>",
  "<Box selectors={{'& > span':{p:4}}}><span/></Box>",
  '4',
  ':number',
  ['paddingTop'],
)
for (const value of ['0', '-0', 'false', 'null', 'undefined', "''", 'NaN']) {
  add(
    `and-${value}`,
    '<Box className="anchor" p={n&&readRight()}/>',
    `<Box className="anchor" p={${value}}/>`,
    value,
  )
  add(
    `runtime-and-${value}`,
    'jsx(Box,{className:"anchor",p:n&&readRight()})',
    `jsx(Box,{className:"anchor",p:${value}})`,
    value,
  )
  add(
    `css-and-${value}`,
    '<div className={css({p:n&&16})}/>',
    `<div className={css({p:${value}})}/>`,
    value,
  )
}
for (const value of [
  'undefined',
  'null',
  'false',
  'true',
  "''",
  'NaN',
  'Infinity',
  '-Infinity',
]) {
  add(
    `absent-padding-${value}`,
    '<Box className="anchor" p={n}/>',
    '<Box className="anchor"/>',
    value,
  )
  fixtures.at(-1).expected = ['7px']
  add(
    `absent-number-${value}`,
    '<Box className="anchor" p={n}/>',
    '<Box className="anchor"/>',
    value,
    ':number',
  )
  fixtures.at(-1).expected = ['7px']
  add(
    `absent-color-${value}`,
    '<Box className="colorAnchor" color={n}/>',
    '<Box className="colorAnchor"/>',
    value,
    '',
    ['color'],
  )
  add(
    `absent-lower-${value}`,
    '<Box className={css({p:3,styleOrder:0})} styleOrder={1} p={n}/>',
    '<Box className={css({p:3,styleOrder:0})} styleOrder={1}/>',
    value,
  )
  add(
    `absent-responsive-${value}`,
    '<Box p={[1,n,2]}/>',
    '<Box p={[1,null,2]}/>',
    value,
  )
  add(
    `absent-selector-${value}`,
    '<Box p={1} _hover={{p:[1,n,2]}}/>',
    '<Box p={1} _hover={{p:[1,null,2]}}/>',
    value,
    '',
    ['paddingTop'],
    true,
  )
  add(
    `absent-runtime-${value}`,
    'jsx(Box,{className:"anchor",p:n})',
    'jsx(Box,{className:"anchor"})',
    value,
  )
}
fixtures.push(...additionalFixtures)
export const source = `import {${components.join(',')},css} from '@devup-ui/react';import {jsx}from 'react/jsx-runtime';
${fixtures.map((fixture, index) => `function dynamic${index}(n${fixture.annotation}){return ${fixture.dynamic}}globalThis.pairs.push([dynamic${index}(${fixture.value}),${fixture.literal}]);`).join('\n')}`
export const importedSource =
  "import {Box}from '@devup-ui/react';import {size}from './types';globalThis.pairs.push([<Box w={size}/>,<Box w={100}/>]);"
export const modules = new Map([
  ['/w42/types.ts', 'export const size:number=readSize();'],
])
export const files = ['/w42/a.tsx', '/w42/b.tsx']
