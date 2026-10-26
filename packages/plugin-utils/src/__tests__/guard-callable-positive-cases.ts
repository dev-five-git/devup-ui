export const intactCallableCases = [
  ['function', 'function Content(){}; export default Content;'],
  ['named default', 'export default function Content(){};'],
  ['anonymous default', 'export default function(){};'],
  ['arrow default', 'export default ()=>0;'],
  [
    'function expression',
    'const Content=function Named(){}; export default Content;',
  ],
  [
    'exported const',
    'export const Content=()=>0; export {Content as default};',
  ],
  [
    'exported function',
    'export function Content(){}; export {Content as default};',
  ],
  [
    'alias chain',
    'function Content(){}; const A=Content; const B=A; export default B;',
  ],
  [
    'long acyclic aliases',
    `function Content(){}; const A0=Content; ${Array.from({ length: 5000 }, (_, index) => `const A${index + 1}=A${index};`).join('')} export default A5000;`,
  ],
  [
    'parameter shadow',
    'function Content(){}; function change(Content){Content=0;} export default Content;',
  ],
  [
    'block shadow',
    'function Content(){}; {let Content; Content=0;} export default Content;',
  ],
  [
    'catch shadow',
    'function Content(){}; try{}catch(Content){Content=0;} export default Content;',
  ],
  [
    'loop shadow',
    'function Content(){}; for(let Content of [0]){Content++;} export default Content;',
  ],
  [
    'nested var shadow',
    'function Content(){}; function change(){var Content; Content=0;} export default Content;',
  ],
  [
    'expression self shadow',
    'function Content(){}; const change=function Content(){Content=0;}; export default Content;',
  ],
  [
    'property write',
    'function Content(){}; Content.value=0; export default Content;',
  ],
  [
    'pattern property write',
    'function Content(){}; [Content.value]=[0]; export default Content;',
  ],
  [
    'pattern key read',
    'function Content(){}; let x; ({[Content]:x}={}); export default Content;',
  ],
  [
    'declaration shadow',
    'function Content(){}; function change(){function Content(){}; Content=0;} export default Content;',
  ],
  [
    'eval parameter shadow',
    'function Content(){}; function change(Content){eval("Content=0");} change(1); export default Content;',
  ],
  [
    'eval block shadow',
    'function Content(){}; {let Content=1; eval("Content=0");} export default Content;',
  ],
  [
    'eval immutable callable',
    'const Content=()=>0; eval("void 0"); export default Content;',
  ],
  [
    'optional indirect eval',
    'function Content(){}; eval?.("void 0"); export default Content;',
  ],
  [
    'sequence indirect eval',
    'function Content(){}; (0,eval)("void 0"); export default Content;',
  ],
] as const
