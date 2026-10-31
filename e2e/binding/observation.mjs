import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'

import { diagnostic } from './diagnostics.mjs'

export function encode(value, seen = new Set()) {
  if (value === undefined) return { tag: 'undefined' }
  if (value === null) return { tag: 'null' }
  const tag = typeof value
  switch (tag) {
    case 'string':
      return { tag, value, bytes: Buffer.byteLength(value, 'utf8') }
    case 'boolean':
      return { tag, value }
    case 'number':
      return {
        tag,
        value: Object.is(value, -0)
          ? '-0'
          : Number.isFinite(value)
            ? value
            : String(value),
      }
    case 'bigint':
    case 'symbol':
      return { tag, value: String(value) }
    case 'function':
      return { tag, name: value.name }
    case 'object': {
      if (seen.has(value)) return { tag: 'cycle' }
      const next = new Set(seen).add(value)
      if (Array.isArray(value))
        return {
          tag: 'array',
          value: Array.from({ length: value.length }, (_, i) =>
            Object.hasOwn(value, i)
              ? encode(value[i], next)
              : { tag: 'absent' },
          ),
        }
      if (value instanceof Map)
        return {
          tag: 'Map',
          value: [...value].map(([k, v]) => [encode(k, next), encode(v, next)]),
        }
      if (value instanceof Set)
        return { tag: 'Set', value: [...value].map((v) => encode(v, next)) }
      return {
        tag,
        value: Reflect.ownKeys(value).map((key) => {
          const d = Object.getOwnPropertyDescriptor(value, key)
          return [
            encode(key, next),
            'value' in d
              ? encode(d.value, next)
              : { tag: 'accessor', get: encode(d.get), set: encode(d.set) },
          ]
        }),
      }
    }
    default:
      throw new Error(`Unsupported encoder type ${tag}`)
  }
}
const failure = (error) =>
  error instanceof Error
    ? {
        tag: 'error',
        class: error.constructor.name,
        name: encode(error.name),
        message: encode(error.message),
        diagnostic: diagnostic(error),
      }
    : { tag: 'thrown', value: encode(error) }
export const capture = (action) => {
  try {
    return { tag: 'return', value: encode(action()) }
  } catch (error) {
    return { tag: 'throw', error: failure(error) }
  }
}
export const hash = (path) =>
  createHash('sha256').update(readFileSync(path)).digest('hex')
