import assert from 'node:assert/strict'

export const pkg = '@devup-ui/react',
  file = 'src/identity.tsx'
export const box = (props) =>
  `import { Box } from '${pkg}'; export const view = <Box ${props} />`
export const source = box('p={3} bg="red" gap={gap}')
export const dependent = `import { Box } from '${pkg}'; import { color } from './tokens'; export const view = <Box color={color} />`
export const theme = {
  colors: { default: { primary: '#123456' }, dark: { primary: '#ffffff' } },
  typography: { heading: [{ fontSize: '12px' }, { fontSize: '20px' }] },
  length: { default: { gutter: ['4px', null, '8px'] } },
  shadow: { default: { card: ['0 1px 2px red', null, '0 2px 4px blue'] } },
}
export const resolved = {
  path: 'src/tokens.ts',
  code: 'export const color = "purple";',
}
const cases = []
const add = (name, run) => cases.push({ name, run })
add('noop-and-plain-js', (h) => {
  for (const code of ['', 'export const value = 1', source]) {
    h.call('hasDevupUI', file, code, pkg)
    h.extract(code)
  }
})
add('scaled-dynamic-repeat-map-modes', (h) => {
  h.extract(source)
  h.extract(source)
  h.extract(box('m={2} w={width} opacity={active ? 1 : 0}'), {
    method: 'codeExtractWithoutSourceMap',
    single: false,
  })
  for (const flags of [
    [false, true, true],
    [true, true, false],
  ])
    h.extract(source, { flags })
})
add('responsive-selector-cascade', (h) =>
  h.extract(
    box(
      'p={[1,null,3]} _hover={{bg:["red","blue"],_focus:{color:"white"}}} selectors={{"& > span":{m:2}}}',
    ),
  ),
)
add('theme-tokens-and-interfaces', (h) => {
  for (const value of [theme, new Map(Object.entries(theme))]) {
    h.call('registerTheme', value)
    h.extract(
      box(
        'color="$primary" typography="heading" w="$gutter" boxShadow="$card"',
      ),
    )
  }
})
add('custom-shorthands-object-map', (h) => {
  for (const value of [
    { insetX: ['left', 'right'] },
    new Map([['insetX', ['left', 'right']]]),
    { insetX: 4 },
  ]) {
    h.call('registerShorthands', value)
    h.extract(box('insetX={[1,null,3]} _hover={{insetX:2}}'))
  }
})
add('styled-emotion-stylex-ve-static-resolver', (h) => {
  const barrel =
    "import { Box } from './barrel'; export const view=<Box p={2}/>;"
  const withoutResolver = h.call('hasDevupUI', file, barrel, pkg, {})
  assert.deepEqual(withoutResolver, {
    tag: 'return',
    value: { tag: 'boolean', value: false },
  })
  const modules = {
    './tokens': resolved,
    './barrel': {
      path: 'src/barrel.ts',
      code: `export { Box } from '${pkg}';`,
    },
    './styles.css': {
      path: 'src/styles.css.ts',
      code: `import { style } from '@vanilla-extract/css'; export const card = style({color:'red',padding:8});`,
    },
  }
  const calls = []
  h.call('setModuleResolver', function (specifier, importer) {
    calls.push([specifier, importer])
    return modules[specifier]
  })
  const withResolver = h.call('hasDevupUI', file, barrel, pkg, {})
  assert.deepEqual(withResolver, {
    tag: 'return',
    value: { tag: 'boolean', value: true },
  })
  for (const code of [
    `import { styled } from '${pkg}'; export const Card = styled('div',{bg:'red',p:2});`,
    `import { css } from '@emotion/react'; export const view = <div css={css({color:'red'})} />;`,
    `import * as stylex from '@stylexjs/stylex'; const s=stylex.create({root:{color:'red'}}); export const view=<div {...stylex.props(s.root)}/>;`,
    `import {card} from './styles.css'; export const view=<div className={card}/>;`,
    dependent,
  ])
    h.extract(code)
  h.note('resolver-calls', calls)
  h.call('setModuleResolver', undefined)
  const clearedResolver = h.call('hasDevupUI', file, barrel, pkg, {})
  assert.deepEqual(clearedResolver, {
    tag: 'return',
    value: { tag: 'boolean', value: false },
  })
})
add('aliases-object-map-null-invalid', (h) => {
  const defaultImport =
    "import Box from '@alias'; export const view=<Box p={2}/>;"
  const namedImport =
    "import { Box } from '@alias'; export const view=<Box p={2}/>;"
  for (const [code, aliases, expected] of [
    [defaultImport, { '@alias': 'Box' }, true],
    [defaultImport, new Map([['@alias', 'Box']]), true],
    [namedImport, { '@alias': null }, true],
    [namedImport, new Map([['@alias', null]]), true],
    [namedImport, { '@alias': undefined }, true],
    [defaultImport, { '@alias': 7 }, false],
    [defaultImport, { '@alias': 'Box', '@invalid': 7 }, false],
    [defaultImport, null, false],
    [defaultImport, undefined, false],
  ]) {
    const result = h.call('hasDevupUI', file, code, pkg, aliases)
    assert.deepEqual(result, {
      tag: 'return',
      value: { tag: 'boolean', value: expected },
    })
  }
  for (const aliases of [
    { '@alias': 'Box' },
    new Map([['@alias', 'Box']]),
    { '@alias': null },
    { '@alias': 7 },
    null,
    undefined,
  ])
    h.extract("import Box from '@alias'; export const view=<Box p={2}/>;", {
      aliases,
    })
})
add('global-css-and-font-faces', (h) =>
  h.extract(
    `import { globalCss } from '${pkg}'; globalCss({'html, body':{m:0},'@font-face':{fontFamily:'Identity',src:'url(identity.woff2)'},body:{color:'red'}});`,
  ),
)
add('keyframes-ordinary-atom-content-boundaries', (h) => {
  h.call('setAtomHoist', 1)
  h.call('importFileRoutes', { [file]: [0] })
  for (const steps of [
    'from:{opacity:0},to:{opacity:1}',
    "'0%, 50%':{transform:'translate(1px, 2px)'},'100%':{transform:'none'}",
    'from:{},to:{opacity:1}',
  ]) {
    h.extract(
      `import {keyframes,Box} from '${pkg}'; const spin=keyframes({${steps}}); export const view=<Box animationName={spin} content={'a;:😀'} _before={{content:'"Ω"'}}/>;`,
    )
    h.extract(
      `import {keyframes,css} from '${pkg}'; const spin=keyframes({${steps}}); export const cls=css({animationName:spin});`,
      { single: false },
    )
  }
})
add('large-same-bucket-128-boxes-keyframes', (h) =>
  h.extract(
    `import {Box,keyframes} from '${pkg}'; const spin=keyframes({from:{opacity:0,transform:'scale(0)'},to:{opacity:1,transform:'scale(1)'}}); export const view=<>${Array.from({ length: 128 }, (_, i) => `<Box p={${i}} m={${127 - i}} color="rgb(${i},2,3)" bg="rgb(3,2,${i})" animationName={spin} _hover={{opacity:${i / 128}}}/>`).join('')}</>;`,
  ),
)
add('atom-hoist-canonical-routes', (h) => {
  h.call('setAtomHoist', 2)
  h.call('importCanonicalMap', { [file]: 'src/root.tsx' })
  for (const routes of [
    { 'src/root.tsx': [0, 3, 3] },
    new Map([['src/root.tsx', new Set([0, 3])]]),
  ]) {
    h.call('importFileRoutes', routes)
    h.extract(source, { single: false })
    h.extract(source, { filename: 'src/root.tsx', single: false })
  }
})
add('seed-unsorted-duplicates', (h) => {
  h.call('seedFileMap', ['src/z.tsx', file, 'src/a.tsx', file])
  h.extract(source, { single: false })
  h.call('seedFileMap', new Set([file]))
})
add('unicode-source-path-content', (h) =>
  h.extract(
    box('title="한글 Ω 😀" content={\'"雪\\\\a"\'} color="rébeccapurple"'),
    { filename: 'src/한글-Ω-😀.tsx' },
  ),
)
add('located-extractor-error', (h) => {
  h.extract(source)
  h.extract(
    `import {css} from '${pkg}';\nexport const cls=css({color:window.name});`,
  )
})
export const extraction = cases
