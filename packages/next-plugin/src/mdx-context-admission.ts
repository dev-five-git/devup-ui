import { parse } from 'node:querystring'

import {
  createMdxContextLifetime,
  type MdxLoaderOccurrence,
} from './mdx-context-lifetime'
import { createMdxDescriptorView } from './mdx-descriptor-view'
import type { MdxJsonParser } from './mdx-options-parser'
import { isMdxRecord } from './mdx-pipeline'

export { MdxContextRefusal } from './mdx-context-lifetime'

export function createMdxContextAdmission(
  jsonParser?: () => MdxJsonParser | undefined,
) {
  const lifetime = createMdxContextLifetime()
  let nativeScope: (() => boolean) | undefined
  function view(
    context: object,
    occurrence: MdxLoaderOccurrence,
    bridge?: {
      readonly compiler?: object
      readonly options?: () => unknown
    },
  ) {
    const access = lifetime.access(context, occurrence)
    const { refuse, guard } = access
    const read = (property: string): unknown => {
      guard(property)
      return Reflect.get(context, property)
    }
    const bind = (property: string) => {
      const native = read(property)
      return (...args: readonly unknown[]) => {
        guard(property)
        if (typeof native !== 'function') return refuse(property)
        return Reflect.apply(native, context, args)
      }
    }
    const bounded = new WeakMap<
      (...args: readonly unknown[]) => unknown,
      (...args: readonly unknown[]) => unknown
    >()
    const capability = (
      name: string,
      method: (...args: readonly unknown[]) => unknown,
    ) => {
      const existing = bounded.get(method)
      if (existing) return existing
      const proxy = new Proxy(
        function (...args: readonly unknown[]) {
          if (name === 'callback' && access.terminal()) return
          guard(name)
          return Reflect.apply(method, undefined, args)
        },
        {
          construct: () => refuse(`${name}.construct`),
          get: (_target, property) =>
            name === 'callback' && property === 'then'
              ? undefined
              : refuse(`${name}.${String(property)}`),
          has: (_target, property) => refuse(`${name}.${String(property)}`),
          ownKeys: () => refuse(`${name}.ownKeys`),
          getOwnPropertyDescriptor: (_target, property) =>
            refuse(`${name}.${String(property)}`),
          getPrototypeOf: () => refuse(`${name}.getPrototypeOf`),
          set: (_target, property) => refuse(`${name}.${String(property)}`),
          defineProperty: (_target, property) =>
            refuse(`${name}.${String(property)}`),
          deleteProperty: (_target, property) =>
            refuse(`${name}.${String(property)}`),
          setPrototypeOf: () => refuse(`${name}.setPrototypeOf`),
          preventExtensions: () => refuse(`${name}.preventExtensions`),
          isExtensible: () => refuse(`${name}.isExtensible`),
        },
      )
      bounded.set(method, proxy)
      return proxy
    }
    const methods = new Map<string, (...args: readonly unknown[]) => unknown>()
    for (const key of [
      'addDependency',
      'addContextDependency',
      'addMissingDependency',
      'addBuildDependency',
      'getDependencies',
      'getContextDependencies',
      'getMissingDependencies',
      'clearDependencies',
      'emitError',
    ])
      methods.set(key, bind(key))
    // loader-runner 16.3.6 assigns dependency and addDependency the same function.
    methods.set(
      'dependency',
      methods.get('addDependency') ?? bind('addDependency'),
    )
    const cacheable = bind('cacheable')
    methods.set('cacheable', (...args) => {
      if (args[0] === false) return refuse('cacheable(false)')
      return cacheable(...args)
    })
    methods.set('getOptions', (schema) => {
      if (schema) return refuse('getOptions(schema)')
      if (bridge?.options) return bridge.options()
      const loaders = read('loaders')
      const slot: unknown = Array.isArray(loaders)
        ? loaders[occurrence.index]
        : undefined
      if (!isMdxRecord(slot)) return refuse('getOptions occurrence')
      const options: unknown = slot.options
      if (typeof options !== 'string') return options ?? {}
      if (options.startsWith('{') && options.endsWith('}')) {
        const parser = jsonParser?.()
        if (!parser) return refuse('getOptions JSON parser unavailable')
        try {
          return parser(options)
        } catch (error) {
          if (!(error instanceof Error)) throw error
          // webpack exposes a plain Error without a cause, not the parser error.
          const nativeError = new Error(
            `Cannot parse string options: ${error.message}`,
          )
          throw nativeError
        }
      }
      return parse(options, '&', '=', { maxKeys: 0 })
    })
    const nativeCallback = read('callback')
    const callbackMethod = (...values: readonly unknown[]) => {
      if (typeof nativeCallback !== 'function') return refuse('callback')
      access.complete()
      return Reflect.apply(nativeCallback, context, values)
    }
    methods.set('callback', callbackMethod)
    const nativeAsync = bind('async')
    methods.set('async', (...args) => {
      const callback = nativeAsync(...args)
      if (typeof callback !== 'function' || callback !== nativeCallback)
        return refuse('async callback')
      access.defer()
      return capability('callback', callbackMethod)
    })
    for (const [name, method] of methods)
      methods.set(name, capability(name, method))
    const scalars = new Set([
      'rootContext',
      'context',
      'resource',
      'resourcePath',
      'resourceQuery',
      'resourceFragment',
      'request',
      'remainingRequest',
      'currentRequest',
      'previousRequest',
      'query',
      'loaderIndex',
      'sourceMap',
      'mode',
    ])
    let descriptorView: unknown
    const descriptors = () => {
      if (descriptorView) return descriptorView
      const loaders = read('loaders')
      if (!Array.isArray(loaders)) return refuse('loaders')
      descriptorView = createMdxDescriptorView(loaders, {
        refuse,
        guard,
        capability,
      })
      return descriptorView
    }
    return new Proxy(
      {},
      {
        get(_target, property) {
          guard(property)
          if (
            bridge?.compiler &&
            ![
              'getOptions',
              'sourceMap',
              'mode',
              '_compiler',
              'context',
              'resourcePath',
            ].includes(String(property))
          )
            return refuse(property)
          if (typeof property === 'string' && methods.has(property))
            return methods.get(property)
          if (property === 'loaderIndex') return occurrence.index
          if (typeof property === 'string' && scalars.has(property))
            return read(property)
          if (property === 'data') return read('data')
          if (property === 'loaders') return descriptors()
          if (property === '_compiler' && bridge?.compiler)
            return bridge.compiler
          return refuse(property)
        },
        has: (_target, property) => refuse(property),
        ownKeys: () => refuse('ownKeys'),
        getOwnPropertyDescriptor: (_target, property) => refuse(property),
        getPrototypeOf: () => refuse('getPrototypeOf'),
        set: (_target, property) => refuse(property),
        defineProperty: (_target, property) => refuse(property),
        deleteProperty: (_target, property) => refuse(property),
        setPrototypeOf: () => refuse('setPrototypeOf'),
        preventExtensions: () => refuse('preventExtensions'),
        isExtensible: () => refuse('isExtensible'),
      },
    )
  }
  return {
    check: lifetime.check,
    close: lifetime.close,
    advance: lifetime.advance,
    run: lifetime.run,
    view,
    admit: (scope: () => boolean) => {
      nativeScope = scope
    },
    scope: () => nativeScope,
  }
}

export type MdxContextAdmission = ReturnType<typeof createMdxContextAdmission>
