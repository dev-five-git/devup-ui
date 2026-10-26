import type { TemplateLiteralType, Type, TypeChecker } from 'typescript'

function isTemplate(type: Type): type is TemplateLiteralType {
  return 'texts' in type
}

type Token = string | null

function pattern(
  checker: TypeChecker,
  type: Type,
): readonly Token[] | undefined {
  // TypeScript template matching consumes UTF-16 code units, not code points.
  if (type.isStringLiteral()) return type.value.split('')
  if (checker.isTypeAssignableTo(checker.getStringType(), type)) return [null]
  if (!isTemplate(type)) return undefined
  const tokens: Token[] = []
  for (const [index, text] of type.texts.entries()) {
    tokens.push(...text.split(''))
    const part = type.types[index]
    if (!part) continue
    const inner = pattern(checker, part)
    if (!inner) return undefined
    tokens.push(...inner)
  }
  return tokens
}

function closure(tokens: readonly Token[], positions: readonly number[]) {
  const result = new Set(positions)
  for (const position of result) {
    if (tokens[position] === null) result.add(position + 1)
  }
  return [...result].sort((a, b) => a - b)
}

function advance(
  tokens: readonly Token[],
  positions: readonly number[],
  character: Token,
) {
  const next = positions.flatMap((position) => {
    const token = tokens[position]
    if (token === null) return [position]
    return token !== undefined && token === character ? [position + 1] : []
  })
  return closure(tokens, next)
}

/** Subset-product reachability: accept both domains and reject every excluded domain. */
function patternsOverlap(
  left: readonly Token[],
  right: readonly Token[],
  excluded: readonly (readonly Token[])[],
): boolean {
  const patterns = [left, right, ...excluded]
  const pending = [
    patterns.map((tokens) => ({ tokens, positions: closure(tokens, [0]) })),
  ]
  const alphabet = new Set(patterns.flat())
  alphabet.add(null)
  const visited = new Set<string>()
  for (let state = pending.pop(); state; state = pending.pop()) {
    const key = state.map(({ positions }) => positions.join(',')).join(';')
    if (visited.has(key)) continue
    visited.add(key)
    if (
      state.every(({ tokens, positions }, index) =>
        index < 2
          ? positions.includes(tokens.length)
          : !positions.includes(tokens.length),
      )
    )
      return true
    for (const character of alphabet) {
      const next = state.map(({ tokens, positions }) => ({
        tokens,
        positions: advance(tokens, positions, character),
      }))
      if (next.slice(0, 2).every(({ positions }) => positions.length))
        pending.push(next)
    }
  }
  return false
}

export function keyDomainsOverlap(
  checker: TypeChecker,
  left: Type,
  right: Type,
): boolean {
  return keyDomainsOverlapExcept(checker, [left, right], [])
}

export function keyDomainsOverlapExcept(
  checker: TypeChecker,
  [left, right]: readonly [Type, Type],
  excluded: readonly Type[],
): boolean {
  if (left === checker.getNeverType() || right === checker.getNeverType())
    return false
  const blockers = excluded.flatMap((type) =>
    type.isUnion() ? type.types : [type],
  )
  if (
    blockers.some(
      (type) =>
        checker.isTypeAssignableTo(left, type) ||
        checker.isTypeAssignableTo(right, type),
    )
  )
    return false
  if (
    !blockers.length &&
    (checker.isTypeAssignableTo(left, right) ||
      checker.isTypeAssignableTo(right, left))
  )
    return true
  if (left.isUnion())
    return left.types.some((part) =>
      keyDomainsOverlapExcept(checker, [part, right], blockers),
    )
  if (right.isUnion())
    return right.types.some((part) =>
      keyDomainsOverlapExcept(checker, [left, part], blockers),
    )
  const a = pattern(checker, left)
  const b = pattern(checker, right)
  const witnessed = () =>
    [a, b].some(
      (tokens) =>
        tokens !== undefined &&
        ['', 'screen', '(display: grid)', '0', '1'].some((filler) => {
          const literal = checker.getStringLiteralType(
            tokens.map((token) => token ?? filler).join(''),
          )
          return (
            checker.isTypeAssignableTo(literal, left) &&
            checker.isTypeAssignableTo(literal, right) &&
            blockers.every((type) => !checker.isTypeAssignableTo(literal, type))
          )
        }),
    )
  if (!a || !b) return witnessed()
  const masks: (readonly Token[])[] = []
  for (const blocker of blockers) {
    const mask = pattern(checker, blocker)
    if (!mask) {
      if (checker.isTypeAssignableTo(blocker, checker.getStringType()))
        return witnessed()
      continue
    }
    masks.push(mask)
  }
  return patternsOverlap(a, b, masks)
}
