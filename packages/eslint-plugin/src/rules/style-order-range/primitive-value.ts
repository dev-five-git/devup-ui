import type { TSESTree } from '@typescript-eslint/utils'

type Primitive = string | number | bigint | boolean | null | undefined
export type StaticValue = { readonly value: Primitive } | null

export const EXACT_MATH = new Set([
  'abs',
  'ceil',
  'clz32',
  'floor',
  'imul',
  'max',
  'min',
  'round',
  'sign',
  'trunc',
])

export function primitive(
  result: { readonly value: unknown } | null,
): StaticValue {
  if (!result) return null
  const value = result.value
  switch (typeof value) {
    case 'string':
    case 'number':
    case 'boolean':
    case 'bigint':
    case 'undefined':
      return { value }
    case 'object':
      return value === null ? { value } : null
    default:
      return null
  }
}

export function binary(
  operator: TSESTree.BinaryExpression['operator'],
  left: Primitive,
  right: Primitive,
): StaticValue {
  if (
    operator === '+' &&
    (typeof left === 'string' || typeof right === 'string')
  )
    return { value: String(left) + String(right) }
  if (typeof left === 'bigint' || typeof right === 'bigint') return null
  const a = Number(left)
  const b = Number(right)
  switch (operator) {
    case '+':
      return { value: a + b }
    case '-':
      return { value: a - b }
    case '*':
      return { value: a * b }
    case '/':
      return { value: a / b }
    case '%':
      return { value: a % b }
    case '**':
      return { value: a ** b }
    case '<<':
      return { value: a << b }
    case '>>':
      return { value: a >> b }
    case '>>>':
      return { value: a >>> b }
    case '|':
      return { value: a | b }
    case '&':
      return { value: a & b }
    case '^':
      return { value: a ^ b }
    case '===':
      return { value: left === right }
    case '!==':
      return { value: left !== right }
    case '<':
      return {
        value:
          typeof left === 'string' && typeof right === 'string'
            ? left < right
            : a < b,
      }
    case '>':
      return {
        value:
          typeof left === 'string' && typeof right === 'string'
            ? left > right
            : a > b,
      }
    case '<=':
      return {
        value:
          typeof left === 'string' && typeof right === 'string'
            ? left <= right
            : a <= b,
      }
    case '>=':
      return {
        value:
          typeof left === 'string' && typeof right === 'string'
            ? left >= right
            : a >= b,
      }
    default:
      return null
  }
}

export function validOrder(value: Primitive): boolean {
  if (typeof value === 'number')
    return Number.isInteger(value) && value >= 1 && value <= 254
  return (
    typeof value === 'string' &&
    validOrder(Number(value)) &&
    String(Number(value)) === value
  )
}
