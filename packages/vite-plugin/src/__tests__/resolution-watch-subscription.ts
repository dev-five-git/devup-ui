import type { EventEmitter } from 'node:events'
import { isAbsolute, normalize } from 'node:path'

import type { ViteDevServer } from 'vite'

export function observeSubscription(
  watcher: EventEmitter & Pick<ViteDevServer['watcher'], 'options'>,
  directory: string,
  signal: AbortSignal,
) {
  const target = normalize(directory).replaceAll('\\', '/')
  const handler: unknown = Reflect.get(watcher, '_nodeFsHandler')
  if (
    !isAbsolute(directory) ||
    typeof handler !== 'object' ||
    handler === null ||
    watcher.options.usePolling ||
    watcher.options.useFsEvents ||
    watcher.options.cwd ||
    Reflect.get(watcher, 'closed') ||
    !Object.isExtensible(handler)
  )
    throw new DOMException(target, 'NotSupportedError')
  const methods = ['_addToNodeFs', '_handleRead', '_watchWithNodeFs'] as const
  const originals = methods.map((name) => {
    const original: unknown = Reflect.get(handler, name)
    const descriptor = Object.getOwnPropertyDescriptor(handler, name)
    if (
      typeof original !== 'function' ||
      (descriptor && (!('value' in descriptor) || !descriptor.configurable))
    )
      throw new DOMException(`${name}: ${target}`, 'NotSupportedError')
    return { name, original, descriptor }
  })
  // These mutable counters belong to the observer, never Chokidar bookkeeping.
  const evidence = {
    case: 'watch-subscription' as const,
    directory: target,
    adds: 0,
    settled: 0,
    scans: 0,
    subscriptions: 0,
    sequence: 0,
    armedAt: 0,
    scannedAt: 0,
    subscribedAt: 0,
    settledAt: 0,
  }
  const completion = Promise.withResolvers<Readonly<typeof evidence>>()
  let active = true
  const fail = (reason: unknown) => completion.reject(reason)
  void completion.promise.then(undefined, fail)
  const check = () => {
    if (
      active &&
      evidence.armedAt &&
      evidence.subscribedAt &&
      evidence.scans &&
      evidence.adds &&
      evidence.adds === evidence.settled
    )
      completion.resolve(Object.freeze({ ...evidence }))
  }
  const restorations: (() => void)[] = []
  const abort = () => fail(signal.reason)
  const dispose = () => {
    active = false
    fail(new DOMException(target, 'AbortError'))
    signal.removeEventListener('abort', abort)
    watcher.off('error', fail)
    for (const restore of restorations) restore()
    restorations.length = 0
  }

  for (const { name, original, descriptor } of originals) {
    const wrapper = new Proxy(original, {
      apply(callable, receiver: unknown, args: unknown[]) {
        const matching =
          active &&
          typeof args[0] === 'string' &&
          normalize(args[0]).replaceAll('\\', '/') === target
        if (matching && name === '_addToNodeFs') {
          evidence.adds++
          evidence.sequence++
        }
        let value: unknown
        try {
          value = Reflect.apply(callable, receiver, args)
        } catch (error) {
          if (matching) fail(error)
          throw error
        }
        if (!matching) return value
        const accepted = {
          _addToNodeFs: value instanceof Promise,
          _handleRead: value === undefined || value instanceof Promise,
          _watchWithNodeFs: value === undefined || typeof value === 'function',
        }
        if (!accepted[name])
          fail(new DOMException(`${name}: ${target}`, 'NotSupportedError'))
        if (name === '_watchWithNodeFs' && typeof value === 'function') {
          evidence.subscriptions++
          evidence.subscribedAt = ++evidence.sequence
        }
        if (value instanceof Promise) {
          void value.then((result: unknown) => {
            if (!active) return
            const completed = {
              _addToNodeFs: result === false,
              _handleRead: result === undefined,
              _watchWithNodeFs: false,
            }
            if (!completed[name])
              fail(new DOMException(`${name}: ${target}`, 'NotSupportedError'))
            const timestamp = {
              _addToNodeFs: 'settledAt',
              _handleRead: 'scannedAt',
              _watchWithNodeFs: 'subscribedAt',
            } as const
            evidence.settled += Number(name === '_addToNodeFs')
            evidence.scans += Number(name === '_handleRead')
            evidence[timestamp[name]] = ++evidence.sequence
            check()
          }, fail)
        }
        check()
        return value
      },
    })
    Object.defineProperty(handler, name, {
      configurable: true,
      writable: true,
      value: wrapper,
    })
    restorations.push(() => {
      if (Reflect.get(handler, name) !== wrapper) return
      if (descriptor) Object.defineProperty(handler, name, descriptor)
      else Reflect.deleteProperty(handler, name)
    })
  }
  watcher.on('error', fail)
  signal.addEventListener('abort', abort, { once: true })
  if (signal.aborted) abort()
  return {
    ready: completion.promise,
    arm() {
      evidence.armedAt = ++evidence.sequence
      check()
    },
    dispose,
    [Symbol.dispose]: dispose,
  }
}
