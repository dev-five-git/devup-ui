'use strict'
const boundary = require('../argument-validation.cjs')()
exports.boundary = boundary
const OP = {
  preflightFailure: 0,
  schema: 1,
  array: 2,
  safeInt: 3,
  bytes: 4,
  debug: 5,
  bigint: 6,
  undefined: 7,
  anyMap: 8,
  entries: 9,
  length: 10,
  index: 11,
  pair: 12,
  field: 13,
  iter: 14,
  next: 15,
  mapStart: 16,
}
exports.OP = OP
const schemaToken = class {}
const accessToken = class {}
function instance(value, type) {
  try {
    return value instanceof type
  } catch {
    return false
  }
}
function debugString(value) {
  if (value == null) return `${value}`
  switch (typeof value) {
    case 'number':
    case 'boolean':
      return `${value}`
    case 'string':
      return `"${value}"`
    case 'symbol': {
      const description = value.description
      return description == null ? 'Symbol' : `Symbol(${description})`
    }
    case 'function': {
      const name = value.name
      return typeof name === 'string' && name.length > 0
        ? `Function(${name})`
        : 'Function'
    }
    default:
      break
  }
  if (Array.isArray(value)) {
    const length = value.length
    let text = '['
    if (length > 0) text += debugString(value[0])
    for (let i = 1; i < length; i++) text += ', ' + debugString(value[i])
    return text + ']'
  }
  const matches = /\[object ([^\]]+)\]/.exec(toString.call(value))
  if (!matches || matches.length < 2) return toString.call(value)
  const name = matches[1]
  if (name === 'Object') {
    try {
      return 'Object(' + JSON.stringify(value) + ')'
    } catch {
      return 'Object'
    }
  }
  if (value instanceof Error)
    return `${value.name}: ${value.message}\n${value.stack}`
  return name
}
function bytes(value) {
  const source = instance(value, Uint8Array)
    ? value
    : instance(value, ArrayBuffer)
      ? new Uint8Array(value)
      : undefined
  if (source === undefined) return undefined
  const copy = new Uint8Array(source.length)
  Uint8Array.prototype.set.call(copy, source)
  return copy
}
exports.withAccess = function withAccess(target, value) {
  let failure, schema
  const access = (operation, value, key) => {
    let wrapped = false
    try {
      let result
      switch (operation) {
        case OP.preflightFailure: {
          const pending = failure
          failure = undefined
          pending(true)
          break
        }
        case OP.schema:
          schema = boundary.fail(key)
          break
        case OP.array:
          result = Array.isArray(value)
          break
        case OP.safeInt:
          result = Number.isSafeInteger(value)
          break
        case OP.bytes:
          result = bytes(value)
          break
        case OP.debug:
          result = `JsValue(${debugString(value)})`
          break
        case OP.bigint:
          result = `${value}`
          break
        case OP.undefined:
          result = undefined
          break
        case OP.anyMap:
          result = !(Symbol.iterator in value) || instance(value, Map)
          break
        case OP.entries: {
          const entries = Object.entries(value)
          result = [entries, entries.length >>> 0]
          break
        }
        case OP.length:
          result = value.length >>> 0
          break
        case OP.index:
          result = value[key]
          break
        case OP.pair:
          if (!boundary.shape(value, true)) return schemaToken
          result = [value[0], value[1]]
          break
        case OP.field: {
          const field = value[key]
          result = [field !== undefined || key in value, field]
          break
        }
        case OP.iter:
        case OP.mapStart: {
          if (!boundary.shape(value)) return schemaToken
          const iteratorSymbol = Symbol.iterator
          wrapped = true
          const method = Reflect.get(value, iteratorSymbol)
          if (typeof method === 'function') {
            const iterator = method.call(value)
            wrapped = false
            result =
              typeof iterator === 'object' &&
              iterator !== null &&
              typeof iterator.next === 'function'
                ? iterator
                : undefined
          }
          if (operation === OP.mapStart) {
            wrapped = false
            if (result !== undefined) result = [1, result]
            else if (key === 1) {
              const entries = Object.entries(value)
              result = [2, entries, entries.length >>> 0]
            } else result = [0]
          }
          break
        }
        case OP.next: {
          wrapped = true
          const next = value.next()
          wrapped = false
          if (!boundary.shape(next)) return schemaToken
          result =
            (+next.done | 0) !== 0 ? [true, undefined] : [false, next.value]
          break
        }
        default:
          throw new Error(`unknown private access operation: ${operation}`)
      }
      return result
    } catch (error) {
      failure = (forgive = false) => {
        if (!wrapped) throw error
        const converted = new Error(String(error))
        if (!forgive) throw converted
      }
      return accessToken
    }
  }
  try {
    return target(value, access, schemaToken, accessToken)
  } catch (error) {
    if (failure) return failure()
    if (schema) throw schema
    throw error
  }
}
