'use strict'
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
  return Array.from(copy)
}
exports.withAccess = function withAccess(run) {
  let failure
  const access = (name, value, key) => {
    let wrapped = false
    try {
      let result
      switch (name) {
        case 'array':
          result = Array.isArray(value)
          break
        case 'safeInt':
          result = Number.isSafeInteger(value)
          break
        case 'bytes':
          result = bytes(value)
          break
        case 'debug':
          result = `JsValue(${debugString(value)})`
          break
        case 'bigint':
          result = String(value)
          break
        case 'undefined':
          result = undefined
          break
        case 'anyMap':
          result = !(Symbol.iterator in value) || instance(value, Map)
          break
        case 'entries':
          result = Object.entries(value)
          break
        case 'length':
          result = value.length >>> 0
          break
        case 'index':
          result = value[Number(key)]
          break
        case 'pair':
          result = [value[0], value[1]]
          break
        case 'field': {
          const field = value[key]
          result = [field !== undefined || key in value, field]
          break
        }
        case 'iter': {
          wrapped = true
          const method = Reflect.get(value, Symbol.iterator)
          if (typeof method !== 'function') break
          const iterator = method.call(value)
          wrapped = false
          result =
            typeof iterator === 'object' &&
            iterator !== null &&
            typeof iterator.next === 'function'
              ? iterator
              : undefined
          break
        }
        case 'next': {
          wrapped = true
          const next = value.next()
          wrapped = false
          result =
            (+next.done | 0) !== 0 ? [true, undefined] : [false, next.value]
          break
        }
        default:
          throw new Error(`unknown private access operation: ${name}`)
      }
      return [true, result]
    } catch (error) {
      failure = () => {
        if (wrapped) throw new Error(String(error))
        throw error
      }
      return [false, undefined]
    }
  }
  try {
    return run(access)
  } catch (error) {
    if (failure) return failure()
    throw error
  }
}
