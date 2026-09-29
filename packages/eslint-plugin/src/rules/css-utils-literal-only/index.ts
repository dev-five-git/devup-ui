import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

import { ImportStorage } from '../../utils/import-storage'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

type Scope = TSESLint.Scope.Scope
type Variable = TSESLint.Scope.Variable
/** Keys read from a binding, `null` for a key only known at runtime */
type Path = (string | null)[]

function findVariable(scope: Scope | null, name: string) {
  for (let current = scope; current; current = current.upper) {
    const variable = current.set.get(name)
    if (variable) return variable
  }
  return undefined
}

/** The globals the build runs: plain data and the functions over it that every engine computes alike */
const GLOBALS = new Set([
  'undefined',
  'NaN',
  'Infinity',
  'Math',
  'String',
  'Number',
  'Boolean',
  'Array',
  'Object',
  'JSON',
  'parseInt',
  'parseFloat',
  'isNaN',
  'isFinite',
  'encodeURIComponent',
  'decodeURIComponent',
  'encodeURI',
  'decodeURI',
])

/** The members of `Math` every engine gives exactly */
const EXACT_MATH = new Set([
  'abs',
  'ceil',
  'floor',
  'round',
  'trunc',
  'sign',
  'max',
  'min',
  'sqrt',
  'fround',
  'imul',
  'clz32',
  'PI',
  'E',
  'LN2',
  'LN10',
  'LOG2E',
  'LOG10E',
  'SQRT2',
  'SQRT1_2',
])

/** Members giving what the locale, the Unicode data of the engine or chance make them */
const UNCERTAIN_MEMBERS = new Set([
  'toLocaleString',
  'toLocaleDateString',
  'toLocaleTimeString',
  'toLocaleUpperCase',
  'toLocaleLowerCase',
  'localeCompare',
  'normalize',
  'random',
])

const MUTATING_METHODS = new Set([
  'push',
  'pop',
  'shift',
  'unshift',
  'splice',
  'sort',
  'reverse',
  'fill',
  'copyWithin',
  'set',
  'delete',
  'clear',
  'add',
])

/** Methods handing the elements of what they are called on to a callback or to the array they return */
const ELEMENT_METHODS = new Set([
  'forEach',
  'map',
  'filter',
  'find',
  'findIndex',
  'findLast',
  'findLastIndex',
  'some',
  'every',
  'reduce',
  'reduceRight',
  'flatMap',
  'flat',
  'concat',
  'slice',
  'toReversed',
  'toSorted',
  'toSpliced',
  'with',
  'values',
  'entries',
  'at',
  'get',
])

/** Methods that only read what they are called on */
const READING_METHODS = new Set([
  'join',
  'includes',
  'indexOf',
  'lastIndexOf',
  'keys',
  'has',
  'toString',
  'valueOf',
  'hasOwnProperty',
  'propertyIsEnumerable',
])

/** Global functions that only read their arguments */
const READING_FUNCTIONS = new Set([
  'String',
  'Number',
  'Boolean',
  'parseInt',
  'parseFloat',
  'isNaN',
  'isFinite',
  'structuredClone',
])

const CHANGING_FUNCTIONS = new Set([
  'assign',
  'defineProperty',
  'defineProperties',
  'setPrototypeOf',
])

const READING_OBJECT_FUNCTIONS = new Set([
  'keys',
  'freeze',
  'seal',
  'preventExtensions',
  'isFrozen',
  'isSealed',
  'getOwnPropertyNames',
  'hasOwn',
])

const TRANSPARENT = new Set<string>([
  AST_NODE_TYPES.TSAsExpression,
  AST_NODE_TYPES.TSSatisfiesExpression,
  AST_NODE_TYPES.TSNonNullExpression,
  AST_NODE_TYPES.ChainExpression,
])

function unwrap(node: TSESTree.Node): TSESTree.Node {
  let current = node
  while (
    current.type === AST_NODE_TYPES.TSAsExpression ||
    current.type === AST_NODE_TYPES.TSSatisfiesExpression ||
    current.type === AST_NODE_TYPES.TSNonNullExpression
  )
    current = current.expression
  return current
}

function isGlobal(node: TSESTree.Node, name: string, scope: Scope) {
  return (
    node.type === AST_NODE_TYPES.Identifier &&
    node.name === name &&
    !findVariable(scope, name)?.defs.length
  )
}

/** The key a member reads, `null` when only the runtime knows it */
function memberKey(member: TSESTree.MemberExpression): string | null {
  if (!member.computed && member.property.type === AST_NODE_TYPES.Identifier)
    return member.property.name
  return member.property.type === AST_NODE_TYPES.Literal
    ? String(member.property.value)
    : null
}

function staticKey(property: TSESTree.Property): string | null {
  if (!property.computed && property.key.type === AST_NODE_TYPES.Identifier)
    return property.key.name
  return property.key.type === AST_NODE_TYPES.Literal
    ? String(property.key.value)
    : null
}

/** Whether a value written as `node` holds nothing code can change: a primitive or a function */
function isPrimitive(node: TSESTree.Node): boolean {
  switch (node.type) {
    case AST_NODE_TYPES.Literal:
      return !('regex' in node && node.regex)
    case AST_NODE_TYPES.TemplateLiteral:
    case AST_NODE_TYPES.UnaryExpression:
    case AST_NODE_TYPES.BinaryExpression:
    case AST_NODE_TYPES.ArrowFunctionExpression:
    case AST_NODE_TYPES.FunctionExpression:
      return true
    case AST_NODE_TYPES.Identifier:
      return node.name === 'undefined'
    default:
      return false
  }
}

/** The values `key` (`null` for any) reads from the literal `node`, `undefined` when the build cannot tell */
function membersOf(
  node: TSESTree.Node,
  key: string | null,
): (TSESTree.Node | null)[] | undefined {
  if (node.type === AST_NODE_TYPES.ObjectExpression) {
    let found: (TSESTree.Node | null)[] = []
    for (const property of node.properties) {
      if (property.type !== AST_NODE_TYPES.Property) return undefined
      const name = staticKey(property)
      if (name === null) return undefined
      if (key === null) found.push(property.value)
      else if (name === key) found = [property.value]
    }
    return found
  }
  if (node.type === AST_NODE_TYPES.ArrayExpression) {
    if (node.elements.some((e) => e?.type === AST_NODE_TYPES.SpreadElement))
      return undefined
    if (key === null) return node.elements
    const element = node.elements[Number(key)]
    return element === undefined ? [] : [element]
  }
  return undefined
}

/** Whether what `path` leads to in the value written as `node` holds nothing code can change */
function reachesOnlyPrimitives(node: TSESTree.Node, path: Path): boolean {
  const value = unwrap(node)
  if (!path.length) return isPrimitive(value)
  const members = membersOf(value, path[0])
  return (
    members !== undefined &&
    members.every(
      (member) =>
        member === null || reachesOnlyPrimitives(member, path.slice(1)),
    )
  )
}

/** Whether the method at `path` of the value written as `init` may read `this` */
function usesThis(init: TSESTree.Node, path: Path): boolean {
  let values: (TSESTree.Node | null)[] | undefined = [init]
  for (const key of path) {
    if (key === null || values?.length !== 1 || !values[0]) return true
    values = membersOf(unwrap(values[0]), key)
  }
  const value = values?.length === 1 && values[0] ? unwrap(values[0]) : null
  if (value?.type === AST_NODE_TYPES.ArrowFunctionExpression) return false
  if (value?.type !== AST_NODE_TYPES.FunctionExpression) return true
  let found = false
  const visit = (node: TSESTree.Node) => {
    if (found) return
    if (node.type === AST_NODE_TYPES.ThisExpression) found = true
    else if (
      node.type !== AST_NODE_TYPES.FunctionExpression &&
      node.type !== AST_NODE_TYPES.FunctionDeclaration
    )
      children(node).forEach(visit)
  }
  children(value.body).forEach(visit)
  return found
}

function children(node: TSESTree.Node): TSESTree.Node[] {
  const found: TSESTree.Node[] = []
  for (const [key, value] of Object.entries(node)) {
    if (key === 'parent') continue
    for (const item of Array.isArray(value) ? value : [value]) {
      if (item && typeof item === 'object' && 'type' in item)
        found.push(item as TSESTree.Node)
    }
  }
  return found
}

/** `[object, member]` when `callee` is a global function, `['', name]` for one that only reads its arguments */
function globalFunction(
  callee: TSESTree.Node,
  scope: Scope,
): [string, string] | null {
  if (
    callee.type === AST_NODE_TYPES.Identifier &&
    READING_FUNCTIONS.has(callee.name) &&
    isGlobal(callee, callee.name, scope)
  )
    return ['', callee.name]
  if (
    callee.type === AST_NODE_TYPES.MemberExpression &&
    !callee.computed &&
    callee.property.type === AST_NODE_TYPES.Identifier &&
    callee.object.type === AST_NODE_TYPES.Identifier &&
    isGlobal(callee.object, callee.object.name, scope)
  )
    return [callee.object.name, callee.property.name]
  return null
}

class Changes {
  private readonly known = new Map<Variable, boolean>()

  constructor(
    private readonly importStorage: ImportStorage,
    private readonly scopeOf: (node: TSESTree.Node) => Scope,
  ) {}

  /** Whether the file changes what `variable` holds, or hands it to code that may, which the build then does not read as a constant */
  isChanged(variable: Variable): boolean {
    const known = this.known.get(variable)
    if (known !== undefined) return known
    // Read while it is checked, so bindings holding each other stop
    this.known.set(variable, false)
    const definition = variable.defs[0]
    const init =
      definition?.type === 'Variable' &&
      definition.node.id.type === AST_NODE_TYPES.Identifier
        ? definition.node.init
        : null
    const changed = variable.references.some(
      (reference) =>
        !reference.init && this.changes(reference.identifier, init ?? null),
    )
    this.known.set(variable, changed)
    return changed
  }

  private changes(identifier: TSESTree.Node, init: TSESTree.Node | null) {
    let node: TSESTree.Node = identifier
    const path: Path = []
    while (node.parent) {
      const parent: TSESTree.Node = node.parent
      if (
        parent.type === AST_NODE_TYPES.MemberExpression &&
        parent.object === node
      )
        path.push(memberKey(parent))
      else if (!TRANSPARENT.has(parent.type)) break
      node = parent
    }
    const parent = node.parent
    const depth = path.length
    // A value handed on changes the binding when it holds what code can change
    const escapes = (at: TSESTree.Node, keys: Path) =>
      init !== null && !reachesOnlyPrimitives(init, keys) && this.escapes(at)
    switch (parent?.type) {
      case AST_NODE_TYPES.CallExpression: {
        const scope = this.scopeOf(parent)
        if (parent.callee === node) {
          if (node.type !== AST_NODE_TYPES.MemberExpression) return false
          const method = path.pop()
          if (method && MUTATING_METHODS.has(method)) return true
          if (method && ELEMENT_METHODS.has(method))
            return escapes(parent, [...path, null])
          if (
            method &&
            (READING_METHODS.has(method) ||
              (init !== null && !usesThis(init, [...path, method])))
          )
            return false
          return (
            init !== null &&
            !reachesOnlyPrimitives(init, path) &&
            !this.inStyle(parent)
          )
        }
        const [object, name] = globalFunction(parent.callee, scope) ?? []
        if (
          object === 'Object' &&
          CHANGING_FUNCTIONS.has(name ?? '') &&
          parent.arguments[0] === node
        )
          return true
        if (
          (object === 'Object' &&
            ['assign', 'values', 'entries'].includes(name ?? '')) ||
          (object === 'Array' && ['from', 'of'].includes(name ?? ''))
        )
          return escapes(parent, [...path, null])
        if (
          object === '' ||
          (object === 'Object' && READING_OBJECT_FUNCTIONS.has(name ?? '')) ||
          ['JSON', 'Math', 'console'].includes(object ?? '') ||
          (object === 'Array' && name === 'isArray')
        )
          return false
        return escapes(parent, path)
      }
      case AST_NODE_TYPES.AssignmentExpression:
        return parent.left === node ? depth > 0 : escapes(parent, path)
      case AST_NODE_TYPES.AssignmentPattern:
        return parent.left === node ? depth > 0 : escapes(parent, path)
      case AST_NODE_TYPES.UnaryExpression:
        return depth > 0 && parent.operator === 'delete'
      case AST_NODE_TYPES.UpdateExpression:
      case AST_NODE_TYPES.ArrayPattern:
      case AST_NODE_TYPES.RestElement:
        return depth > 0
      case AST_NODE_TYPES.ForInStatement:
        return depth > 0 && parent.left === node
      case AST_NODE_TYPES.ForOfStatement:
        return parent.left === node
          ? depth > 0
          : escapes(parent, [...path, null])
      case AST_NODE_TYPES.SpreadElement:
        return escapes(parent, [...path, null])
      case AST_NODE_TYPES.Property:
        if (parent.parent.type === AST_NODE_TYPES.ObjectPattern)
          return depth > 0 && parent.value === node
        return parent.value === node && escapes(parent, path)
      case AST_NODE_TYPES.JSXExpressionContainer:
        return (
          parent.parent.type === AST_NODE_TYPES.JSXAttribute &&
          parent.parent.name.type === AST_NODE_TYPES.JSXIdentifier &&
          parent.parent.name.name === 'ref' &&
          escapes(parent, path)
        )
      case AST_NODE_TYPES.VariableDeclarator:
        return parent.init === node && escapes(parent, path)
      case AST_NODE_TYPES.TemplateLiteral:
        return (
          parent.parent.type === AST_NODE_TYPES.TaggedTemplateExpression &&
          escapes(parent, path)
        )
      case AST_NODE_TYPES.ArrayExpression:
      case AST_NODE_TYPES.NewExpression:
      case AST_NODE_TYPES.ReturnStatement:
      case AST_NODE_TYPES.YieldExpression:
      case AST_NODE_TYPES.ExportDefaultDeclaration:
      case AST_NODE_TYPES.TaggedTemplateExpression:
      case AST_NODE_TYPES.ArrowFunctionExpression:
        return escapes(parent, path)
      default:
        return false
    }
  }

  /** Whether code at `node` is read by a style API, which never runs it */
  private inStyle(node: TSESTree.Node) {
    for (let current: TSESTree.Node | undefined = node; current;) {
      if (this.importStorage.checkContextType(current)) return true
      current = current.parent
    }
    return false
  }

  /** Whether a value handed on at `node` goes where code may change it: anywhere but the style APIs, a top-level `const` nothing changes, or what the module exports */
  private escapes(node: TSESTree.Node): boolean {
    if (this.inStyle(node)) return false
    const freezes = (call: TSESTree.CallExpression) => {
      const [object, name] =
        globalFunction(call.callee, this.scopeOf(call)) ?? []
      return (
        object === 'Object' &&
        ['freeze', 'seal', 'preventExtensions'].includes(name ?? '')
      )
    }
    let holder: TSESTree.Node = node
    while (
      holder.parent &&
      (holder.type === AST_NODE_TYPES.Property ||
        holder.type === AST_NODE_TYPES.ObjectExpression ||
        holder.type === AST_NODE_TYPES.ArrayExpression ||
        holder.type === AST_NODE_TYPES.SpreadElement ||
        TRANSPARENT.has(holder.type) ||
        (holder.type === AST_NODE_TYPES.CallExpression && freezes(holder)))
    )
      holder = holder.parent
    const topLevel = (declaration: TSESTree.Node | undefined) =>
      declaration?.parent?.type === AST_NODE_TYPES.Program ||
      declaration?.parent?.type === AST_NODE_TYPES.ExportNamedDeclaration
    if (
      holder.type === AST_NODE_TYPES.VariableDeclarator &&
      holder.parent.kind === 'const' &&
      topLevel(holder.parent) &&
      holder.id.type === AST_NODE_TYPES.Identifier
    ) {
      const variable = findVariable(this.scopeOf(holder), holder.id.name)
      return variable !== undefined && this.isChanged(variable)
    }
    if (holder.type === AST_NODE_TYPES.ExportDefaultDeclaration) return false
    return !(
      holder.type === AST_NODE_TYPES.AssignmentExpression &&
      holder.parent.type === AST_NODE_TYPES.ExpressionStatement &&
      holder.parent.parent.type === AST_NODE_TYPES.Program &&
      isCommonjsExport(holder.left)
    )
  }
}

function isCommonjsExport(node: TSESTree.Node) {
  if (node.type !== AST_NODE_TYPES.MemberExpression) return false
  const object = node.object
  if (object.type === AST_NODE_TYPES.MemberExpression)
    return (
      object.object.type === AST_NODE_TYPES.Identifier &&
      object.object.name === 'module' &&
      memberKey(object) === 'exports'
    )
  return (
    object.type === AST_NODE_TYPES.Identifier &&
    (object.name === 'exports' ||
      (object.name === 'module' && memberKey(node) === 'exports'))
  )
}

/** Whether calling the member `callee` calls into an import: the build never runs what another module declares */
function callsImport(callee: TSESTree.MemberExpression, scope: Scope) {
  let root: TSESTree.Node = callee.object
  while (root.type === AST_NODE_TYPES.MemberExpression) root = root.object
  return (
    root.type === AST_NODE_TYPES.Identifier &&
    findVariable(scope, root.name)?.defs[0]?.type === 'ImportBinding'
  )
}

class Values {
  private readonly pure = new Map<TSESTree.Node, boolean>()

  constructor(
    private readonly changes: Changes,
    private readonly scopeOf: (node: TSESTree.Node) => Scope,
  ) {}

  /** Whether reading member `name` of `object` gives the same on every engine and page */
  private exactMember(
    object: TSESTree.Node,
    name: string | null,
    scope: Scope,
  ) {
    if (name !== null && UNCERTAIN_MEMBERS.has(name)) return false
    return !(isGlobal(object, 'Math', scope) && !EXACT_MATH.has(name ?? ''))
  }

  /** Whether the build knows `node`'s value: a literal, a constant, or what exact built-ins and the file's own pure functions compute from them */
  isStaticValue(node: TSESTree.Node, scope: Scope, seen: Set<string>): boolean {
    const all = (nodes: (TSESTree.Node | null)[]) =>
      nodes.every(
        (item) =>
          item !== null &&
          item.type !== AST_NODE_TYPES.SpreadElement &&
          this.isStaticValue(item, scope, seen),
      )
    switch (node.type) {
      case AST_NODE_TYPES.Literal:
        return !('regex' in node && node.regex)
      case AST_NODE_TYPES.TemplateLiteral:
        return all(node.expressions)
      case AST_NODE_TYPES.BinaryExpression:
        return (
          node.operator !== '**' &&
          node.left.type !== AST_NODE_TYPES.PrivateIdentifier &&
          all([node.left, node.right])
        )
      case AST_NODE_TYPES.LogicalExpression:
        return all([node.left, node.right])
      case AST_NODE_TYPES.ConditionalExpression:
        return all([node.test, node.consequent, node.alternate])
      case AST_NODE_TYPES.UnaryExpression:
        return (
          node.operator === '-' &&
          this.isStaticValue(node.argument, scope, seen)
        )
      case AST_NODE_TYPES.TSAsExpression:
      case AST_NODE_TYPES.TSSatisfiesExpression:
        return this.isStaticValue(node.expression, scope, seen)
      case AST_NODE_TYPES.ArrayExpression:
        return all(node.elements)
      case AST_NODE_TYPES.ObjectExpression:
        return node.properties.every(
          (property) =>
            property.type === AST_NODE_TYPES.Property &&
            property.kind === 'init' &&
            !property.computed &&
            (property.value.type === AST_NODE_TYPES.ArrowFunctionExpression ||
            property.value.type === AST_NODE_TYPES.FunctionExpression
              ? this.isPure(property.value)
              : this.isStaticValue(property.value, scope, seen)),
        )
      case AST_NODE_TYPES.MemberExpression:
        return (
          (node.computed
            ? this.isStaticValue(node.property, scope, seen)
            : this.exactMember(node.object, memberKey(node), scope)) &&
          (isGlobal(node.object, 'Math', scope) ||
            this.isStaticValue(node.object, scope, seen))
        )
      case AST_NODE_TYPES.CallExpression:
        return this.isStaticCallee(node, scope, seen) && all(node.arguments)
      case AST_NODE_TYPES.Identifier:
        return (
          ['undefined', 'NaN', 'Infinity'].includes(node.name) ||
          this.isStaticBinding(node.name, scope, seen)
        )
      default:
        return false
    }
  }

  /** Whether the build runs the callee of `call`: an exact built-in, a method of a value it knows, or a pure function the file declares */
  isStaticCallee(
    call: TSESTree.CallExpression,
    scope: Scope,
    seen: Set<string>,
  ): boolean {
    const callee = call.callee
    if (callee.type === AST_NODE_TYPES.MemberExpression) {
      const key = memberKey(callee)
      if (
        (callee.computed && callee.property.type !== AST_NODE_TYPES.Literal) ||
        (key === 'toString' && call.arguments.length > 0) ||
        !this.exactMember(callee.object, key, scope)
      )
        return false
      if (callsImport(callee, scope)) return false
      const object = unwrap(callee.object)
      return (
        (object.type === AST_NODE_TYPES.Identifier &&
          GLOBALS.has(object.name) &&
          !findVariable(scope, object.name)?.defs.length) ||
        this.isStaticValue(object, scope, seen)
      )
    }
    if (callee.type !== AST_NODE_TYPES.Identifier) return false
    const variable = findVariable(scope, callee.name)
    const definition = variable?.defs[0]
    if (!variable || !definition) return GLOBALS.has(callee.name)
    if (!['module', 'global'].includes(variable.scope.type)) return false
    if (definition.type === 'FunctionName') return this.isPure(definition.node)
    return (
      definition.type === 'Variable' &&
      definition.parent.kind === 'const' &&
      definition.node.init !== null &&
      (definition.node.init.type === AST_NODE_TYPES.ArrowFunctionExpression ||
        definition.node.init.type === AST_NODE_TYPES.FunctionExpression) &&
      !this.changes.isChanged(variable) &&
      this.isPure(definition.node.init)
    )
  }

  isStaticBinding(name: string, scope: Scope, seen: Set<string>): boolean {
    if (seen.has(name)) return false
    const variable = findVariable(scope, name)
    const definition = variable?.defs[0]
    if (!variable || !definition || this.changes.isChanged(variable))
      return false
    if (definition.type === 'ImportBinding') return true
    if (
      definition.type !== 'Variable' ||
      definition.parent.kind !== 'const' ||
      !['module', 'global'].includes(variable.scope.type) ||
      definition.node.id.type !== AST_NODE_TYPES.Identifier ||
      !definition.node.init
    )
      return false
    seen.add(name)
    return this.isStaticValue(definition.node.init, variable.scope, seen)
  }

  /** Whether the build runs `fn`: it does only plain computing over what it is given, constants and other such functions */
  private isPure(fn: TSESTree.Node): boolean {
    const known = this.pure.get(fn)
    if (known !== undefined) return known
    // Read while it is checked, so functions calling each other stop
    this.pure.set(fn, true)
    const pure = this.checkPure(fn)
    this.pure.set(fn, pure)
    return pure
  }

  private checkPure(fn: TSESTree.Node): boolean {
    let pure = true
    const outside = (node: TSESTree.Node) => {
      let root = node
      while (root.type === AST_NODE_TYPES.MemberExpression) root = root.object
      if (root.type !== AST_NODE_TYPES.Identifier) return false
      const variable = findVariable(this.scopeOf(root), root.name)
      return (
        variable !== undefined &&
        ['module', 'global'].includes(variable.scope.type)
      )
    }
    const visit = (node: TSESTree.Node) => {
      if (!pure) return
      // Types are erased before the code runs
      if (
        node.type === AST_NODE_TYPES.TSAsExpression ||
        node.type === AST_NODE_TYPES.TSSatisfiesExpression ||
        node.type === AST_NODE_TYPES.TSNonNullExpression
      ) {
        visit(node.expression)
        return
      }
      if (node.type.startsWith('TS')) return
      switch (node.type) {
        case AST_NODE_TYPES.ThisExpression:
        case AST_NODE_TYPES.Super:
        case AST_NODE_TYPES.MetaProperty:
        case AST_NODE_TYPES.NewExpression:
        case AST_NODE_TYPES.ClassExpression:
        case AST_NODE_TYPES.ClassDeclaration:
        case AST_NODE_TYPES.TryStatement:
        case AST_NODE_TYPES.AwaitExpression:
        case AST_NODE_TYPES.YieldExpression:
        case AST_NODE_TYPES.ImportExpression:
        case AST_NODE_TYPES.JSXElement:
        case AST_NODE_TYPES.JSXFragment:
          pure = false
          return
        case AST_NODE_TYPES.Literal:
          pure = !('regex' in node && node.regex)
          return
        case AST_NODE_TYPES.FunctionDeclaration:
        case AST_NODE_TYPES.FunctionExpression:
        case AST_NODE_TYPES.ArrowFunctionExpression:
          if (node.async || ('generator' in node && node.generator)) {
            pure = false
            return
          }
          break
        case AST_NODE_TYPES.BinaryExpression:
          if (node.operator === '**') pure = false
          break
        case AST_NODE_TYPES.AssignmentExpression:
          if (
            node.operator === '**=' ||
            (node.left.type === AST_NODE_TYPES.MemberExpression &&
              outside(node.left))
          )
            pure = false
          break
        case AST_NODE_TYPES.UpdateExpression:
          if (outside(node.argument)) pure = false
          break
        case AST_NODE_TYPES.UnaryExpression:
          if (node.operator === 'delete' && outside(node.argument)) pure = false
          break
        case AST_NODE_TYPES.Property:
          if (
            node.kind !== 'init' ||
            (node.parent.type === AST_NODE_TYPES.ObjectPattern &&
              UNCERTAIN_MEMBERS.has(staticKey(node) ?? ''))
          )
            pure = false
          break
        case AST_NODE_TYPES.MemberExpression:
          if (
            !this.exactMember(node.object, memberKey(node), this.scopeOf(node))
          )
            pure = false
          break
        case AST_NODE_TYPES.CallExpression:
          if (
            node.callee.type === AST_NODE_TYPES.MemberExpression &&
            ((node.callee.computed &&
              node.callee.property.type !== AST_NODE_TYPES.Literal) ||
              (memberKey(node.callee) === 'toString' &&
                node.arguments.length > 0) ||
              callsImport(node.callee, this.scopeOf(node)))
          )
            pure = false
          break
        case AST_NODE_TYPES.Identifier:
          pure = this.reads(node)
          break
      }
      children(node).forEach(visit)
    }
    visit(fn)
    return pure
  }

  /** Whether the build runs code reading `identifier` in a function it runs */
  private reads(identifier: TSESTree.Identifier): boolean {
    const parent = identifier.parent
    if (
      (parent.type === AST_NODE_TYPES.MemberExpression &&
        parent.property === identifier &&
        !parent.computed) ||
      (parent.type === AST_NODE_TYPES.Property &&
        parent.key === identifier &&
        !parent.computed)
    )
      return true
    const scope = this.scopeOf(identifier)
    const variable = findVariable(scope, identifier.name)
    const definition = variable?.defs[0]
    if (!variable || !definition) return GLOBALS.has(identifier.name)
    if (!['module', 'global'].includes(variable.scope.type)) return true
    if (this.changes.isChanged(variable)) return false
    switch (definition.type) {
      case 'ImportBinding':
        return !(
          parent.type === AST_NODE_TYPES.CallExpression &&
          parent.callee === identifier
        )
      case 'FunctionName':
        return this.isPure(definition.node)
      case 'Variable': {
        const init = definition.node.init
        if (definition.parent.kind !== 'const' || !init) return false
        return init.type === AST_NODE_TYPES.ArrowFunctionExpression ||
          init.type === AST_NODE_TYPES.FunctionExpression
          ? this.isPure(init)
          : this.isStaticValue(init, variable.scope, new Set())
      }
      default:
        return false
    }
  }
}

export const cssUtilsLiteralOnly = createRule({
  name: 'css-utils-literal-only',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      cssUtilsLiteralOnly:
        'CSS utils should only be used with values known at build time: literals, constants, or what exact built-ins and the functions of this file compute from them.',
    },
    type: 'problem',
    docs: {
      description:
        'CSS utils should only be used with values known at build time.',
    },
  },
  create(context) {
    const importStorage = new ImportStorage()
    const scopeOf = (node: TSESTree.Node) => context.sourceCode.getScope(node)
    const changes = new Changes(importStorage, scopeOf)
    const values = new Values(changes, scopeOf)
    let devupContext: TSESTree.CallExpression | null = null
    return {
      ImportDeclaration(node) {
        importStorage.addImportByDeclaration(node)
      },
      CallExpression(node) {
        if (
          importStorage.checkContextType(node) === 'UTIL' &&
          node.arguments.length === 1 &&
          node.arguments[0].type === AST_NODE_TYPES.ObjectExpression
        ) {
          devupContext = node
        }
      },
      'CallExpression:exit'(node) {
        if (devupContext === node) {
          devupContext = null
        }
      },
      Identifier(node) {
        if (!devupContext || node.name === 'undefined') return

        const an = context.sourceCode
          .getAncestors(node)
          .slice(context.sourceCode.getAncestors(devupContext).length)
        const scope = scopeOf(node)
        // A binding the value declares itself, such as a callback's parameter
        const declared = findVariable(scope, node.name)?.defs[0]?.name.range
        if (
          declared &&
          declared[0] >= devupContext.range[0] &&
          declared[1] <= devupContext.range[1]
        )
          return

        let call: TSESTree.CallExpression | null = null
        let member: TSESTree.MemberExpression | null = null
        for (const ancestor of an) {
          switch (ancestor.type) {
            case AST_NODE_TYPES.Property:
              if ([...an, node].indexOf(ancestor.key) !== -1) return
              break
            case AST_NODE_TYPES.ConditionalExpression:
              if ([...an, node].indexOf(ancestor.test) !== -1) return
              break
            case AST_NODE_TYPES.MemberExpression:
              if ([...an, node].indexOf(ancestor.property) !== -1) return
              member = ancestor
              break
            case AST_NODE_TYPES.CallExpression:
              if ([...an, node].indexOf(ancestor.callee) !== -1) {
                if (ancestor === devupContext) return
                call = ancestor
              }
              break
          }
        }

        if (
          call
            ? values.isStaticCallee(call, scope, new Set())
            : member
              ? values.isStaticValue(member, scope, new Set())
              : values.isStaticBinding(node.name, scope, new Set())
        )
          return

        context.report({
          node,
          messageId: 'cssUtilsLiteralOnly',
        })
      },
    }
  },
})
