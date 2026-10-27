import { expect, it } from 'bun:test'
import { parseAst } from 'vite'

import {
  compiledFacts,
  type GuardObservation,
} from '../../../plugin-utils/src/guard-facts'
import { MdxDefaultProvenanceError, validateMdxDefault } from '../mdx-default'

it.each([
  ['assignment', 'function Content(){}; Content=0; export default Content;'],
  [
    'alias after write',
    'function Content(){}; Content=0; const Alias=Content; export default Alias;',
  ],
  ['named default', 'export default function Content(){}; Content=0;'],
  [
    'object assignment',
    'function Content(){}; ({value:Content}={value:0}); export default Content;',
  ],
  [
    'array assignment',
    'function Content(){}; [Content]=[0]; export default Content;',
  ],
  ['update', 'function Content(){}; Content++; export default Content;'],
  [
    'loop write',
    'function Content(){}; for(Content of [0]){}; export default Content;',
  ],
  [
    'nested write',
    'function Content(){}; function change(){Content=0;} change(); export default Content;',
  ],
  [
    'alias chain',
    'function Content(){}; Content=0; const A=Content; const B=A; export default B;',
  ],
  [
    'self write',
    'function Content(){Content=0;} Content(); export default Content;',
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
    'direct eval',
    'function Content(){}; eval("Content=0"); export default Content;',
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
])('refuses non-callable default provenance after %s', (_name, source) => {
  // Given parsed JavaScript whose exported function binding is actually overwritten.
  const ast = parseAst(source)
  let observation: GuardObservation | undefined
  // When both current-pipeline provenance predicates inspect it.
  compiledFacts(ast, (value) => {
    observation = value
  })
  // Then neither predicate may certify the default as an ordinary local callable.
  expect(observation?.defaultCallable).toBe(false)
  expect(() => validateMdxDefault(ast, source, '/content.mdx')).toThrow(
    MdxDefaultProvenanceError,
  )
})

it('refuses direct intrinsic eval in the prevalidator independently', () => {
  // Given: parsed MDX output whose intrinsic eval can replace Content.
  const source =
    'function Content(){}; eval("Content=0"); export default Content;'
  const ast = parseAst(source)
  // When: pre-extraction provenance is checked without a final-facts assertion first.
  // Then: this boundary must refuse independently of the final guard.
  expect(() => validateMdxDefault(ast, source, '/content.mdx')).toThrow(
    MdxDefaultProvenanceError,
  )
})

it.each([
  'function Content(){}; function change(Content){eval("Content=0");} change(1); export default Content;',
  'function Content(){}; {let Content=1; eval("Content=0");} export default Content;',
  'const Content=()=>0; eval("void 0"); export default Content;',
  'function Content(){}; eval?.("void 0"); export default Content;',
])(
  'keeps callable proof for an eval call without access to its mutable binding: %s',
  (source) => {
    // Given: an immutable callable, a shadowed binding or indirect eval.
    const ast = parseAst(source)
    let observation: GuardObservation | undefined
    // When: both provenance gates inspect the same parsed source.
    compiledFacts(ast, (value) => {
      observation = value
    })
    // Then: these calls do not revoke the exported binding's proof.
    expect(observation?.defaultCallable).toBe(true)
    expect(() => validateMdxDefault(ast, source, '/content.mdx')).not.toThrow()
  },
)

it('keeps an unrelated shadowed write from invalidating the exported binding', () => {
  const source =
    'function Content(){}; function change(Content){Content=0;} export default Content;'
  const ast = parseAst(source)
  let observation: GuardObservation | undefined
  compiledFacts(ast, (value) => {
    observation = value
  })
  expect(observation?.defaultCallable).toBe(true)
  expect(() => validateMdxDefault(ast, source, '/content.mdx')).not.toThrow()
})

it('proves long acyclic aliases without recursive stack growth', () => {
  // Given a flat, valid module with a long chain to an ordinary local callable.
  const source = `function Content(){}; const A0=Content; ${Array.from({ length: 5000 }, (_, index) => `const A${index + 1}=A${index};`).join('')} export default A5000;`
  const ast = parseAst(source)
  let observation: GuardObservation | undefined
  // When both existing provenance predicates follow the exact alias chain.
  compiledFacts(ast, (value) => {
    observation = value
  })
  // Then the known callable succeeds without a depth waiver or stack increase.
  expect(observation?.defaultCallable).toBe(true)
  expect(() => validateMdxDefault(ast, source, '/content.mdx')).not.toThrow()
})

it.each(['\n', '\r\n', '\r', '\u2028', '\u2029'])(
  'locates rejected MDX defaults after %j',
  (newline) => {
    // Given legal JS line separators following a multibyte and surrogate-pair prefix.
    const source = `const label='한😀';${newline}export default 0;`
    // When the actual pre-extraction provenance validator rejects the default.
    // Then its compiled-output coordinate is the authored second-line export.
    expect(() =>
      validateMdxDefault(parseAst(source), source, '/content.mdx'),
    ).toThrow('/content.mdx:2:1:')
  },
)
