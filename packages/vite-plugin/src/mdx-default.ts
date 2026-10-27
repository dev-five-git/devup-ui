import { compiledFacts } from '@devup-ui/plugin-utils'

interface Node {
  readonly type: string
  readonly [field: string]: unknown
}

function node(value: unknown): Node | undefined {
  return value !== null &&
    typeof value === 'object' &&
    'type' in value &&
    typeof value.type === 'string'
    ? { ...value, type: value.type }
    : undefined
}

function name(value: unknown): string | undefined {
  const identifier = node(value)
  switch (identifier?.type) {
    case 'Identifier':
      return typeof identifier.name === 'string' ? identifier.name : undefined
    case 'Literal':
      return typeof identifier.value === 'string' ? identifier.value : undefined
    default:
      return undefined
  }
}

export class MdxDefaultProvenanceError extends Error {
  readonly name = 'MdxDefaultProvenanceError'
  constructor(
    readonly filename: string,
    code: string,
    readonly position: number,
  ) {
    const lines = code.slice(0, position).split(/\r\n|[\n\r\u2028\u2029]/)
    super(
      `${filename}:${lines.length}:${(lines.at(-1)?.length ?? 0) + 1}: (in compiled MDX) default export provenance cannot use a forwarded or unknown value at build time: the MDX compiler must preserve an ordinary local content component, not replace its default with a compile-time export; move the value into JS/TS`,
    )
  }
}

export function validateMdxDefault(
  ast: unknown,
  code: string,
  filename: string,
): void {
  const root = node(ast)
  if (root?.type !== 'Program' || !Array.isArray(root.body))
    throw new MdxDefaultProvenanceError(filename, code, 0)
  const body = root.body
    .map(node)
    .filter((item): item is Node => item !== undefined)
  let defaultCallable = false
  compiledFacts(ast, (value) => {
    defaultCallable = value.defaultCallable
  })
  for (const statement of body) {
    let hasDefault = statement.type === 'ExportDefaultDeclaration'
    if (
      statement.type === 'ExportAllDeclaration' &&
      name(statement.exported) === 'default'
    )
      hasDefault = true
    else if (
      statement.type === 'ExportNamedDeclaration' &&
      Array.isArray(statement.specifiers)
    ) {
      for (const item of statement.specifiers) {
        const specifier = node(item)
        if (name(specifier?.exported) === 'default') {
          hasDefault = true
        }
      }
    }
    if (hasDefault && !defaultCallable)
      throw new MdxDefaultProvenanceError(
        filename,
        code,
        typeof statement.start === 'number' ? statement.start : 0,
      )
  }
}
