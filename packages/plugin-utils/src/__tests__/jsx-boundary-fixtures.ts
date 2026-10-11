export const jsxBoundaryFixtures: readonly {
  readonly code: string
  readonly jsx: boolean
  readonly edges: readonly string[]
}[] = [
  ...[
    'a<b',
    'a>b',
    'a<<b',
    'a<<=b',
    'a>>b',
    'a>>>b',
    'a>>=b',
    'a>>>=b',
    '1 <<foo',
    'f<T>(x)',
    'f<Array<T>>(x)',
    'new Map<string, number>()',
    'a[0]<b',
    'f(x)<b',
    '1<b',
  ].map((expression) => ({
    code: `const x = ${expression}; import('./real'); require('./required');`,
    jsx: true,
    edges: ['dynamic:./real', 'static:./required'],
  })),
  ...[
    '<T,>(x: T)=>x',
    '<T extends U>(x: T)=>x',
    '<T extends {x: U}>(x: T)=>x',
    '<T extends Array<U>>(x: T)=>x',
    '<T,>(x: T): T=>x',
    '<T extends U>(x = import("./inside"))=>x',
    '<T /* marker */,>(x: T)=>x',
    '<T /* marker */ extends /* next */ U>(x: T)=>x',
    '<const T,>(x: T)=>x',
    '<const /* marker */ T,>(x: T)=>x',
    '<T // marker\n,>(x: T)=>x',
    '<T extends // marker\n U>(x: T)=>x',
    '<T = U,>(x: T)=>x',
  ].map((expression) => ({
    code: `const fn = ${expression}; import('./real'); require('./required');`,
    jsx: true,
    edges: expression.includes('./inside')
      ? ['dynamic:./inside', 'dynamic:./real', 'static:./required']
      : ['dynamic:./real', 'static:./required'],
  })),
  {
    code: "const x = <T>value; type X = Map<string, Array<T>>; import('./real'); require('./required');",
    jsx: false,
    edges: ['dynamic:./real', 'static:./required'],
  },
  ...[
    { prefix: 'const x = ', suffix: ';' },
    { prefix: 'function f(){return ', suffix: '}' },
    { prefix: 'const x = (', suffix: ');' },
    { prefix: 'const x = ok ? ', suffix: ' : null;' },
    { prefix: 'const x = ok ? null : ', suffix: ';' },
    { prefix: 'const x = ok && ', suffix: ';' },
    { prefix: 'const x = ok || ', suffix: ';' },
    { prefix: 'const x = {node: ', suffix: '};' },
    { prefix: 'const x = [', suffix: '];' },
    { prefix: 'const x = () => ', suffix: ';' },
    { prefix: 'const x = 0 << ', suffix: ';' },
    { prefix: 'const x = 0 <= ', suffix: ';' },
    { prefix: 'const x = 0 === ', suffix: ';' },
    { prefix: 'const x = ok ?? ', suffix: ';' },
    { prefix: 'const x = 0 ** ', suffix: ';' },
  ].map(({ prefix, suffix }) => ({
    code: `${prefix}<Comp attr={import('./attribute')} label="import('phantom')">require('phantom'){require('./child')}</Comp>${suffix} import('./real');`,
    jsx: true,
    edges: ['dynamic:./attribute', 'static:./child', 'dynamic:./real'],
  })),
  {
    code: "const jsx = <T extends=\"U\">import('phantom'){import('./real')}</T>; require('./required');",
    jsx: true,
    edges: ['dynamic:./real', 'static:./required'],
  },
]
