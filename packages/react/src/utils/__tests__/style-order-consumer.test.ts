import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'bun:test'

const directory = dirname(fileURLToPath(import.meta.url))
const root = resolve(directory, '../../../../..')
const imports = [
  `import {css} from ${JSON.stringify(join(directory, '../css'))};`,
  `import {globalCss} from ${JSON.stringify(join(directory, '../global-css'))};`,
  `import {keyframes} from ${JSON.stringify(join(directory, '../keyframes'))};`,
  `import {createGlobalStyle} from ${JSON.stringify(join(directory, '../create-global-style'))};`,
  `import {Box} from ${JSON.stringify(join(directory, '../../components/Box'))};`,
].join('\n')

function check(source: string) {
  const temporary = mkdtempSync(join(directory, 'consumer-'))
  const fixture = join(temporary, 'contract.tsx')
  try {
    writeFileSync(fixture, imports + '\n' + source)
    return Bun.spawnSync({
      cmd: [
        process.execPath,
        join(root, 'node_modules/@typescript/native/bin/tsc'),
        '--ignoreConfig',
        '--noEmit',
        '--strict',
        '--skipLibCheck',
        '--jsx',
        'react-jsx',
        '--moduleResolution',
        'bundler',
        '--module',
        'esnext',
        '--target',
        'esnext',
        '--types',
        'react',
        '--typeRoots',
        join(root, 'packages/react/node_modules/@types'),
        fixture,
      ],
      stdout: 'pipe',
      stderr: 'pipe',
    })
  } finally {
    rmSync(temporary, { recursive: true })
  }
}

describe('public style order contracts when checked by native TypeScript', () => {
  it('accepts supported metadata and scalar template shapes', () => {
    const source = `
      declare const active: boolean;
      const parts = [{color:'red', styleOrder:2}, false, null, undefined, ['class']] as const;
      css(parts, {styleOrder:'254'}, {'style-order':'2'});
      css({styleOrder:active?1:2, _hover:{styleOrder:'3', color:'blue'}});
      css\`style-order:\${active?1:2};color:\${'red'}\`;
      css\`color:\${false}\${null}\${undefined}\`;
      globalCss({styleOrder:2,body:{color:'red',styleOrder:'3'},'s:hover':{color:'blue'},style:{color:'red'}});
      globalCss({'style-order':'2',body:{'style-order':3,color:'red'}});
      globalCss({'@media print':{styleOrder:2,body:{styleOrder:3,color:'red'}}});
      globalCss({_media:{print:{styleOrder:2,body:{color:'red'}}}});
      globalCss({styleOrder:2,fontFaces:[{fontFamily:'Test',src:'url(test.woff)'}]});
      globalCss\`body{style-order:\${2};color:red}\`;
      createGlobalStyle({styleOrder:'2',body:{styleOrder:3,color:'red'}});
      createGlobalStyle\`body{style-order:\${2}}\`;
      keyframes({from:{opacity:0},to:{opacity:1}});
      keyframes\`from{opacity:\${0}}to{opacity:\${1}}\`;
      const element = <Box styleOrder="254" _hover={{styleOrder:2}} />;
    `
    const result = check(source)
    expect(new TextDecoder().decode(result.stdout)).toBe('')
    expect(result.exitCode).toBe(0)
  })

  const rejected = [
    'css({styleOrder:0})',
    'css({styleOrder:255})',
    'css({styleOrder:1.5})',
    'css({styleOrder:"02"})',
    'css({"style-order":"+2"})',
    'css({color:123})',
    'css({_hover:{styleOrder:0}})',
    'css(true)',
    'css(()=>({color:"red"}))',
    'css`style-order:${()=>2}`',
    'css`style-order:${{order:2}}`',
    'globalCss({styleOrder:0,body:{color:"red"}})',
    'globalCss({styleOrder:2,body:{color:123}})',
    'globalCss({body:{styleOrder:"02"}})',
    'globalCss`style-order:${()=>2}`',
    'globalCss`style-order:${{order:2}}`',
    'createGlobalStyle({styleOrder:0,body:{color:"red"}})',
    'createGlobalStyle`style-order:${()=>2}`',
    'keyframes`from{opacity:${{value:0}}}`',
    'const frame={opacity:0,styleOrder:2};keyframes({from:frame})',
    'const frame={opacity:0,"style-order":2};keyframes({from:frame})',
    'const frames={from:{opacity:0},styleOrder:2};keyframes(frames)',
    'const face={fontFamily:"Test",src:"url(test)",styleOrder:2};globalCss({fontFaces:[face]})',
    'const face={fontFamily:"Test",src:"url(test)","style-order":2};globalCss({fontFaces:[face]})',
    'const element=<Box styleOrder="02" />',
  ]
  it.each(rejected)('rejects unsupported input %s', (source) => {
    const result = check(source)
    expect(result.exitCode).not.toBe(0)
    expect(new TextDecoder().decode(result.stdout)).toMatch(/error TS\d+:/)
  })
})
