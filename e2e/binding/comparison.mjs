import { readFileSync } from 'node:fs'
import { join } from 'node:path'

function firstByte(left, right) {
  let offset = 0
  while (
    offset < Math.min(left.length, right.length) &&
    left[offset] === right[offset]
  )
    offset++
  return {
    offset,
    leftLength: left.length,
    rightLength: right.length,
    leftHex: left
      .subarray(Math.max(0, offset - 32), offset + 64)
      .toString('hex'),
    rightHex: right
      .subarray(Math.max(0, offset - 32), offset + 64)
      .toString('hex'),
  }
}
function leaf(left, right, path = '$') {
  if (
    typeof left !== typeof right ||
    left === null ||
    right === null ||
    typeof left !== 'object'
  )
    return Object.is(left, right)
      ? null
      : {
          field: path,
          leftType: typeof left,
          rightType: typeof right,
          ...firstByte(
            Buffer.from(
              typeof left === 'string'
                ? left
                : (JSON.stringify(left) ?? 'undefined'),
            ),
            Buffer.from(
              typeof right === 'string'
                ? right
                : (JSON.stringify(right) ?? 'undefined'),
            ),
          ),
        }
  if (
    left.tag === 'object' &&
    right.tag === 'object' &&
    left.value.length === right.value.length
  ) {
    for (let i = 0; i < left.value.length; i++) {
      const [key, value] = left.value[i],
        [otherKey, otherValue] = right.value[i]
      const diff =
        leaf(key, otherKey, `${path}.key[${i}]`) ??
        leaf(value, otherValue, `${path}.${key.value ?? i}`)
      if (diff) return diff
    }
    return null
  }
  for (const key of new Set([...Object.keys(left), ...Object.keys(right)])) {
    if (!Object.hasOwn(left, key) || !Object.hasOwn(right, key))
      return {
        field: `${path}.${key}`,
        leftPresent: Object.hasOwn(left, key),
        rightPresent: Object.hasOwn(right, key),
      }
    const diff = leaf(left[key], right[key], `${path}.${key}`)
    if (diff) return diff
  }
  return null
}
export function compare(output, problems) {
  const comparisons = []
  for (const [a, b] of [
    ['wasm-1', 'wasm-2'],
    ['wasm-1', 'wasm-3'],
    ['native-1', 'native-2'],
    ['native-1', 'native-3'],
    ...Array.from({ length: 3 }, (_, i) => [
      `wasm-${i + 1}`,
      `native-${i + 1}`,
    ]),
  ]) {
    const comparison = { left: a, right: b, equal: false }
    comparisons.push(comparison)
    try {
      const left = readFileSync(join(output, a, 'records.jsonl')),
        right = readFileSync(join(output, b, 'records.jsonl'))
      comparison.equal = left.equals(right)
      if (!comparison.equal) {
        comparison.firstDiff = firstByte(left, right)
        const line =
          left
            .subarray(0, comparison.firstDiff.offset)
            .toString('utf8')
            .split('\n').length - 1
        const l = left.toString('utf8').split('\n')[line],
          r = right.toString('utf8').split('\n')[line]
        comparison.record = line
        comparison.fieldDiff = leaf(JSON.parse(l), JSON.parse(r))
        problems.push({
          comparison: `${a}:${b}`,
          reason: 'literal record bytes differ',
        })
      }
    } catch (error) {
      problems.push({ comparison: `${a}:${b}`, reason: error.message })
    }
  }
  return comparisons
}
