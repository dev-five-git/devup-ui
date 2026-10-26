export const overwrittenCallableCases = [
  ['assignment', 'function Content(){}; Content=0; export default Content;'],
  [
    'alias chain',
    'function Content(){}; Content=0; const A=Content; const B=A; export default B;',
  ],
  ['named default', 'export default function Content(){}; Content=0;'],
  [
    'default specifier',
    'function Content(){}; Content=0; export {Content as default};',
  ],
  [
    'object pattern',
    'function Content(){}; ({value:Content}={value:0}); export default Content;',
  ],
  [
    'array pattern',
    'function Content(){}; [Content]=[0]; export default Content;',
  ],
  [
    'nested pattern',
    'function Content(){}; ({value:[Content=0]}={value:[]}); export default Content;',
  ],
  [
    'array rest',
    'function Content(){}; [...Content]=[]; export default Content;',
  ],
  [
    'object rest',
    'function Content(){}; ({...Content}={}); export default Content;',
  ],
  ['update', 'function Content(){}; Content++; export default Content;'],
  [
    'for of',
    'function Content(){}; for(Content of [0]){}; export default Content;',
  ],
  [
    'for in',
    'function Content(){}; for(Content in {x:0}){}; export default Content;',
  ],
  [
    'compound assignment',
    'function Content(){}; Content+=0; export default Content;',
  ],
  [
    'loop pattern',
    'function Content(){}; for({value:Content} of [{value:0}]){}; export default Content;',
  ],
  [
    'nested write',
    'function Content(){}; function change(){Content=0;} change(); export default Content;',
  ],
  [
    'self write',
    'function Content(){Content=0;} Content(); export default Content;',
  ],
  [
    'parameter initializer',
    'function Content(){}; function change(x=(Content=0)){} change(); export default Content;',
  ],
  [
    'computed pattern key',
    'function Content(){}; let x; ({[Content=0]:x}={}); export default Content;',
  ],
  [
    'pattern member expression',
    'function Content(){}; const holder={}; [holder[Content=0]]=[1]; export default Content;',
  ],
  [
    'object member expression',
    'function Content(){}; const holder={}; ({x:holder[Content=0]}={x:1}); export default Content;',
  ],
  [
    'after export',
    'function Content(){}; export {Content as default}; Content=0;',
  ],
  [
    'direct eval',
    'function Content(){}; eval("Content=0"); export default Content;',
  ],
  [
    'dynamic direct eval',
    'function Content(){}; const code="Content=0"; eval(code); export default Content;',
  ],
  [
    'direct eval alias',
    'function Content(){}; eval("Content=0"); const A=Content; const B=A; export default B;',
  ],
  [
    'direct eval named default',
    'export default function Content(){}; eval("Content=0");',
  ],
  [
    'nested direct eval',
    'function Content(){}; function change(){eval("Content=0");} change(); export default Content;',
  ],
  [
    'self direct eval',
    'function Content(){eval("Content=0");} Content(); export default Content;',
  ],
  [
    'direct eval plus shadow',
    'function Content(){}; function change(Content){eval("Content=0");} change(1); eval("Content=0"); export default Content;',
  ],
] as const
