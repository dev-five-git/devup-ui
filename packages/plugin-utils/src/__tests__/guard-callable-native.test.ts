import { createRequire } from 'node:module'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

import {
  callableLocals,
  callableWrites,
  localCallable,
} from '../guard-callable'
import { compiledFacts, type GuardObservation } from '../guard-facts'
import { createScopes, type GuardNode, guardNode } from '../guard-nodes'
import { overwrittenCallableCases } from './guard-callable-negative-cases'
import { intactCallableCases } from './guard-callable-positive-cases'

const require = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const parser: unknown = require('next/dist/compiled/acorn')
if (
  !parser ||
  typeof parser !== 'object' ||
  !('parse' in parser) ||
  typeof parser.parse !== 'function'
)
  throw new TypeError('Installed Webpack parser must expose parse')
const parse = parser.parse

it.each(overwrittenCallableCases)(
  'refuses callable provenance when %s overwrites the binding',
  (_name, source) => {
    // Given: real parser output with a write to the exported module binding.
    const ast: unknown = parse(source, {
      ecmaVersion: 'latest',
      sourceType: 'module',
    })
    let observation: GuardObservation | undefined
    // When: the final-facts observer evaluates local default provenance.
    compiledFacts(ast, (value) => {
      observation = value
    })
    // Then: the original function declaration is no longer callable evidence.
    expect(observation?.defaultCallable).toBe(false)
  },
)

it.each([
  [
    'shadowed local eval',
    'function Content(){}; function run(){function eval(){}; eval("Content=0");} run();',
    true,
  ],
  [
    'shadowed eval writer',
    'function Content(){}; function run(){function eval(){Content=0;} eval();} run();',
    false,
  ],
] as const)(
  'respects script lexical bindings for %s',
  (_name, source, callable) => {
    // Given: real script grammar, where a local binding named eval is legal.
    const root = guardNode(
      parse(source, { ecmaVersion: 'latest', sourceType: 'script' }),
    )
    if (!root) throw new TypeError('Installed parser must expose a program')
    const locals = new Map<string, GuardNode>()
    for (const statement of root.fields.get('body') ?? [])
      callableLocals(statement, locals)
    const { scopes } = createScopes(root)
    // When: the same write collector inspects the lexical bindings.
    for (const [node, scope] of scopes) callableWrites(node, scope, locals)
    // Then: only the visible writer revokes the outer function proof.
    expect(localCallable(locals.get('Content'), locals)).toBe(callable)
  },
)

it.each(intactCallableCases)(
  'preserves callable provenance when %s leaves the binding intact',
  (_name, source) => {
    // Given: a local function whose binding has no relevant write.
    const ast: unknown = parse(source, {
      ecmaVersion: 'latest',
      sourceType: 'module',
    })
    let observation: GuardObservation | undefined
    // When: final facts resolve the default through lexical locals.
    compiledFacts(ast, (value) => {
      observation = value
    })
    // Then: unrelated bindings and object properties do not revoke callability.
    expect(observation?.defaultCallable).toBe(true)
  },
)

it.each([
  ['missing default', 'function Content(){};'],
  ['literal', 'export default 0;'],
  ['unknown', 'export default missing;'],
  ['alias cycle', 'const A=B; const B=A; export default A;'],
  ['forwarded', 'export {default} from "content";'],
  ['mutable local', 'let Content=()=>0; export default Content;'],
  ['class', 'export default class Content{};'],
] as const)('does not invent callable provenance for %s', (_name, source) => {
  // Given: an unsupported default shape or no default.
  const ast: unknown = parse(source, {
    ecmaVersion: 'latest',
    sourceType: 'module',
  })
  let observation: GuardObservation | undefined
  // When: the real-parser AST is observed.
  compiledFacts(ast, (value) => {
    observation = value
  })
  // Then: only the established local callable subset is certified.
  expect(observation?.defaultCallable).toBe(false)
})
