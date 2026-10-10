import { expect, it } from 'bun:test'

import { scanImports } from '../import-scanner'

it.each([
  {
    code: "const jsx = <div>{<span>import('phantom')</span> && import('./real')}</div>;",
    edges: ['dynamic:./real'],
  },
  {
    code: "const jsx = <div>{(<span>import('phantom')</span>) && import('./real')}</div>;",
    edges: ['dynamic:./real'],
  },
  {
    code: "const jsx = <div>{<span>require('phantom')</span> && require('./required')}</div>;",
    edges: ['static:./required'],
  },
  {
    code: "const jsx = <div>{(<span>require('phantom')</span>) && require('./required')}</div>;",
    edges: ['static:./required'],
  },
  {
    code: "const jsx = <Comp value={<span>import('phantom')</span>}>require('phantom'){import('./real')}</Comp>;",
    edges: ['dynamic:./real'],
  },
  {
    code: "const jsx = <Comp value={<span>require('phantom')</span>}>import('phantom'){require('./required')}</Comp>;",
    edges: ['static:./required'],
  },
  {
    code: "const jsx = <Comp value={(<span/> && require('./required'))} label=\"import('phantom')\">require('phantom'){import('./real')}</Comp>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{ok ? <span>import('phantom')</span> : import('./real')}{require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{ok ? import('./real') : <span>require('phantom')</span>}{require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{({node: <span>import('phantom')</span>, leaf: import('./real')})}{require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{[<span>import('phantom')</span>, import('./real'), require('./required')]}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{call(<span>require('phantom')</span>, import('./real'), require('./required'))}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{<><span>import('phantom')</span></> && import('./real')}{require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{<span/> && import('./real')}require('phantom'){require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{<span>{<b>import('phantom')</b> && import('./real')}require('phantom')</span> && require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <Comp value={<Other value={<span>import('phantom')</span>}>require('phantom'){import('./real')}</Other>}>import('phantom'){require('./required')}</Comp>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <Comp value={<span/>} next={import('./real')} label='require(&quot;phantom&quot;)'/>; require('./required');",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <Comp {...{node: <span>import('phantom')</span>, leaf: import('./real')}}>require('phantom'){require('./required')}</Comp>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{(() => { const node = <span>import('phantom')</span>; return import('./real') })()}{require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{(() => <span>require('phantom')</span>)() && import('./real')}{require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{`${<span>import('phantom')</span> && import('./real')} require('phantom')`}{require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "const jsx = <div>{!<span>import('phantom')</span> && import('./real')}{require('./required')}</div>;",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "async function f() { return <div>{await <span>import('phantom')</span> && import('./real')}{require('./required')}</div> }",
    edges: ['dynamic:./real', 'static:./required'],
  },
  {
    code: "function* f() { return <div>{(yield <span>require('phantom')</span>) && import('./real')}{require('./required')}</div> }",
    edges: ['dynamic:./real', 'static:./required'],
  },
])(
  'keeps only real nested JSX edges when scanning $code',
  ({ code, edges }) => {
    // Given: valid TSX, checked without evaluating its imports or expressions.
    new Bun.Transpiler({ loader: 'tsx' }).transformSync(code)
    // When: scan the original source, not the compiler output.
    const actual = scanImports(code, true)
      .map((edge) => `${edge.kind}:${edge.specifier}`)
      .sort()
    // Then: fixture-derived dependencies exclude every text/quoted phantom.
    expect(actual).toEqual([...edges])
  },
)

it.each([
  '~',
  '+',
  '-',
  'void ',
  'typeof ',
  'delete ',
  '0 + ',
  '0 - ',
  '0 * ',
  '0 / ',
  '0 % ',
  '0 ** ',
  '0 ^ ',
  '0 & ',
  '0 | ',
  '0 << ',
  '0 > ',
  '0 < ',
  '0 in ',
  '0 instanceof ',
])('restores parent code when JSX follows expression prefix %s', (prefix) => {
  // Given: valid unary/binary operand syntax surrounding nested JSX.
  const code = `const jsx = <div>{${prefix}<span>import('phantom')</span> && import('./real')}{require('./required')}</div>;`
  new Bun.Transpiler({ loader: 'tsx' }).transformSync(code)
  // When: scan the original operand expression.
  const actual = scanImports(code, true)
  // Then: expression operators neither leak text nor hide following dependencies.
  expect(actual).toEqual([
    { kind: 'dynamic', specifier: './real' },
    { kind: 'static', specifier: './required' },
  ])
})
