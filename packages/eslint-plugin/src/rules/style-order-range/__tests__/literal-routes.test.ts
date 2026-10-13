import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { styleOrderRange } from '../index'

const filename = 'src/literal-routes.tsx'
const devup =
  'import { css, styled, globalCss, keyframes, Box } from "@devup-ui/react";'
const classes = [
  [devup, 'css`VALUE`'],
  [devup, 'css("VALUE")'],
  [devup, 'css([`VALUE`])'],
  [devup, 'styled.div`VALUE`'],
  [devup, 'styled("div", `VALUE`)'],
  [devup, 'styled("div")`VALUE`'],
  [
    devup,
    'styled.div.attrs({text:"style-order:0"}).withConfig({text:"style-order:0"})`VALUE`',
  ],
  ['import styled from "@emotion/styled";', 'styled.div`VALUE`'],
  ['import styled from "styled-components";', 'styled.div`VALUE`'],
  [
    'import styled from "styled-components";const factory=styled.div.attrs({text:"style-order:0"});',
    'factory`VALUE`',
  ],
  ['import { css } from "@emotion/react";', 'css`VALUE`'],
  ['import { css } from "@emotion/css";', 'css`VALUE`'],
  ['import { css } from "styled-components";', 'css`VALUE`'],
  ['import { css } from "@devup-ui/react/compat";', 'css`VALUE`'],
  ['import { style as css } from "@vanilla-extract/css";', 'css`VALUE`'],
  ['import { style as css } from "@vanilla-extract/css";', 'css("VALUE")'],
  ['import * as D from "@devup-ui/react"; const c = D.css;', 'c`VALUE`'],
  [
    'import { ClassNames } from "@emotion/react";',
    '<ClassNames>{({css:c})=>c`VALUE`}</ClassNames>',
  ],
  ['import { css } from "@emotion/react";', '<div css="VALUE"/>'],
  [devup, '<Box css={`VALUE`}/>'],
  [
    'import { jsx } from "@emotion/react/jsx-runtime";',
    'jsx("div",{css:`VALUE`})',
  ],
  [devup, 'css({selectors:{styleOrder:`VALUE`}})'],
  [devup, 'css({_media:{print:`VALUE`}})'],
] as const
const globals = [
  [devup, 'globalCss`body{VALUE}`'],
  [devup, 'globalCss({body:`VALUE`})'],
  [
    'import { createGlobalStyle as g } from "styled-components";',
    'g`body{VALUE}`',
  ],
  [
    'import { Global } from "@emotion/react";',
    '<Global styles={`body{VALUE}`}/>',
  ],
  [
    'import { Global } from "@devup-ui/react/compat";',
    '<Global {...{styles:`body{VALUE}`}}/>',
  ],
  [
    'import { Global } from "@emotion/react";import { jsx } from "react/jsx-runtime";',
    'jsx(Global,{styles:`body{VALUE}`})',
  ],
  [
    'import { globalStyle as g } from "@vanilla-extract/css";',
    'g("body",`VALUE`)',
  ],
] as const
const noEffect = [
  [devup, 'keyframes`from{VALUE}`'],
  [devup, 'keyframes({from:`VALUE`})'],
  ['import { keyframes as k } from "styled-components";', 'k`to{VALUE}`'],
  ['import { keyframes as k } from "@emotion/react";', 'k`to{VALUE}`'],
  [devup, 'globalCss`@font-face{VALUE}`'],
  [devup, 'globalCss`@keyframes spin{from{VALUE}}`'],
  [devup, 'globalCss({fontFaces:[`VALUE`]})'],
  [devup, 'globalCss({"@font-face":`VALUE`})'],
  [devup, 'globalCss({"@keyframes spin":{from:`VALUE`}})'],
  [devup, 'css`@FONT-FACE{VALUE}`'],
] as const

describe('literal text routes when consumed by supported APIs', () => {
  const tester = new RuleTester({
    languageOptions: { parserOptions: { ecmaFeatures: { jsx: true } } },
  })
  tester.run('direct and compatibility routes', styleOrderRange, {
    valid: [
      ...[...classes, ...globals].map(([prefix, shape]) => ({
        filename,
        code: prefix + shape.replace('VALUE', 'style-order:2;color:red'),
      })),
      {
        filename: 'src/styles.css.ts',
        code: 'import {style} from "@vanilla-extract/css";style("style-order:0");style`style-order:0`;',
      },
      {
        filename,
        code:
          devup +
          'css`/* ${"style-order:0"} */content:"${"style-order:0"}";background:url(${"style-order:0"});--data:{${"style-order:0"}};color:red`',
      },
      { filename, code: devup + 'css`style-order:${+"02"}/*tail*/;color:red`' },
      {
        filename,
        code: devup + 'globalCss`body{style-order:1${2};color:red}`',
      },
      {
        filename,
        code: devup + 'styled.div`style-order:${p=>p.on?2:3};color:red`',
      },
      {
        filename,
        code:
          devup +
          'styled.div`style-order:${function(p){const order=p.on?2:3;return order;}};color:red`',
      },
      {
        filename,
        code:
          devup +
          'styled.div`style-order:${p=>{if(p.on){return 2}else{return 3}}}`',
      },
      {
        filename,
        code:
          devup +
          'styled.div`style-order:${p=>{if(true)return 2;else return 0}}`',
      },
      {
        filename,
        code:
          devup + 'const order=p=>p.on?2:3;styled.div`style-order:${order}`',
      },
      {
        filename,
        code: devup + 'const factory=factory;factory`style-order:0`;',
      },
      {
        filename,
        code:
          devup +
          'const attrs=styled.div.attrs;attrs({styleOrder:0,text:"style-order:0"})`style-order:2`;',
      },
      {
        filename,
        code:
          devup +
          'const config=styled.div.withConfig;const alias=config;alias({styleOrder:0})`style-order:2`;',
      },
      {
        filename,
        code:
          devup +
          'styled.div`style-order:${p=>{if(p.on){const n=2;return n}else{return 3}}}`',
      },
      {
        filename,
        code:
          devup + 'styled.div`style-order:${p=>{if(p.on)return 2;return 3}}`',
      },
      {
        filename,
        code:
          devup + 'styled.div`style-order:${p=>{if(false)return 0;return 2}}`',
      },
      {
        filename,
        code:
          devup +
          'css`style-order:${on?2:3};styleOrder:254; &:hover{style-order:3;color:red}`',
      },
      { filename, code: 'import {css} from "other";css`style-order:0`;' },
      {
        filename,
        code: 'import {css} from "@devup-ui/react";const tag=x=>x;css({content:tag`style-order:0`})',
      },
    ],
    invalid: [
      ...classes.map(([prefix, shape]) => ({
        filename,
        code: prefix + shape.replace('VALUE', 'style-order:0;color:red'),
        errors: [{ messageId: 'styleOrderRange' }],
      })),
      ...globals.map(([prefix, shape]) => ({
        filename,
        code:
          prefix + shape.replace('VALUE', 'style-order:${on?2:3};color:red'),
        errors: [{ messageId: 'globalOrder' }],
      })),
      ...noEffect.map(([prefix, shape]) => ({
        filename,
        code: prefix + shape.replace('VALUE', 'styleOrder:runtime;color:red'),
        errors: [{ messageId: 'unsupportedOrder' }],
      })),
      ...[
        'css`style-order:1${runtime};color:red`',
        'css`style-order:1${" 2"};color:red`',
        'css`style-order:1${on?2:255};color:red`',
        'css`style-order:${p=>2};color:red`',
        'styled.div`style-order:${p=>p.on?2:0};color:red`',
        'styled.div`style-order:${p=>{let order=2;return order;}};color:red`',
        'styled.div`style-order:${async p=>2};color:red`',
        'styled.div`style-order:${function*(){return 2}};color:red`',
        'styled.div`style-order:${p=>{return;}};color:red`',
        'styled.div`style-order:${p=>{}}`',
        'styled.div`style-order:${p=>{if(p.on)return 2}}`',
        'styled.div`style-order:${p=>{if(p.on)throw 2;else return 3}}`',
        'styled.div`style-order:${p=>{if(p.on)return 0;else return 3}}`',
        'css`${{styleOrder:0}}color:red`',
        'css`&:hover{${"style-order:0"}color:red}`',
        'globalCss(on&&`body{style-order:2;color:red}`)',
        'globalCss`body{style-order:1${on?2:3};color:red}`',
        'css({selectors:{"&":css`style-order:0`}})',
      ].map((tail) => ({
        filename,
        code: devup + tail,
        errors: [
          {
            messageId: tail.startsWith('globalCss')
              ? 'globalOrder'
              : 'styleOrderRange',
          },
        ],
      })),
    ],
  })
})
