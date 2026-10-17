import { EventEmitter } from 'node:events'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { observeSubscription } from './resolution-watch-subscription'

const directory = resolve('subscription-fixture')
const names = ['_addToNodeFs', '_handleRead', '_watchWithNodeFs'] as const
function fixture() {
  const results: Record<string, unknown> = {
    _addToNodeFs: Promise.resolve(false),
    _handleRead: Promise.resolve(),
    _watchWithNodeFs: () => {
      throw new DOMException(directory, 'InvalidStateError')
    },
  }
  const calls: unknown[] = []
  const method = (name: string) =>
    function (this: unknown, ...args: unknown[]) {
      calls.push([this, args])
      const value = results[name]
      if (value instanceof DOMException) throw value
      return value
    }
  const handler = {
    _addToNodeFs: method(names[0]),
    _handleRead: method(names[1]),
    _watchWithNodeFs: method(names[2]),
  }
  const watcher = Object.assign(new EventEmitter(), {
    _nodeFsHandler: handler,
    options: { usePolling: false, useFsEvents: false },
  })
  return { handler, watcher, results, calls, abort: new AbortController() }
}

it('preserves identity and fences every overlapping enrollment', async () => {
  // Given prototype methods and separately controlled enrollment and scan promises.
  const f = fixture()
  Object.setPrototypeOf(f.handler, { ...f.handler })
  for (const name of names) Reflect.deleteProperty(f.handler, name)
  const original = f.handler._addToNodeFs
  const first = Promise.withResolvers<boolean>()
  const second = Promise.withResolvers<boolean>()
  const scan = Promise.withResolvers<void>()
  using observer = observeSubscription(f.watcher, directory, f.abort.signal)
  f.results[names[0]] = first.promise
  const args = [directory, true, 0, 0, undefined]
  expect(f.handler._addToNodeFs(...args)).toBe(first.promise)
  f.results[names[0]] = second.promise
  expect(f.handler._addToNodeFs(directory, false, {}, 1)).toBe(second.promise)
  f.results[names[1]] = scan.promise
  expect(f.handler._handleRead(directory)).toBe(scan.promise)
  const listener = () => undefined
  expect(f.handler._watchWithNodeFs(directory, listener)).toBe(
    f.results[names[2]],
  )
  expect(f.calls).toEqual([
    [f.handler, args],
    [f.handler, [directory, false, {}, 1]],
    [f.handler, [directory]],
    [f.handler, [directory, listener]],
  ])
  expect(f.handler._addToNodeFs.length).toBe(original.length)
  const receipts: unknown[] = []
  void observer.ready.then((receipt) => receipts.push(receipt))
  // When the scan and first enrollment finish, then the second enrollment still blocks readiness.
  observer.arm()
  scan.resolve()
  first.resolve(false)
  await first.promise
  expect(receipts).toHaveLength(0)
  second.resolve(false)
  const receipt = await observer.ready
  expect(receipt).toMatchObject({
    directory: directory.replaceAll('\\', '/'),
    adds: 2,
    settled: 2,
    scans: 1,
    subscriptions: 1,
  })
  expect(receipt.scannedAt).toBeGreaterThan(receipt.subscribedAt)
  expect(receipt.settledAt).toBeGreaterThan(receipt.armedAt)
  observer.dispose()
  expect(f.handler._addToNodeFs).toBe(original)
  expect(names.some((name) => Object.hasOwn(f.handler, name))).toBe(false)
})

it.each(['scan', 'native', 'add', 'arm', 'directory'])(
  'waits when %s evidence is missing',
  async (missing) => {
    // Given incomplete exact-directory evidence, including a throttled read or absent closer.
    const f = fixture()
    using observer = observeSubscription(f.watcher, directory, f.abort.signal)
    const path =
      missing === 'directory' ? resolve(directory, 'child') : directory
    f.results[names[1]] = missing === 'scan' ? undefined : f.results[names[1]]
    f.results[names[2]] = missing === 'native' ? undefined : f.results[names[2]]
    f.handler._handleRead(path)
    f.handler._watchWithNodeFs(path)
    if (missing !== 'add') f.handler._addToNodeFs(path)
    const receipts: unknown[] = []
    void observer.ready.then(
      (receipt) => receipts.push(receipt),
      () => undefined,
    )
    // When available operations finish, then an incomplete or unarmed fence cannot succeed.
    if (missing !== 'arm') observer.arm()
    await Promise.resolve()
    expect(receipts).toHaveLength(0)
    f.abort.abort()
    await expect(observer.ready).rejects.toBe(f.abort.signal.reason)
  },
)

it.each(['abort', 'error', 'dispose', 'replacement'] as const)(
  'restores ownership after %s',
  async (action) => {
    // Given an own descriptor and an enrollment that cannot finish during teardown.
    const f = fixture()
    const pending = Promise.withResolvers<boolean>()
    const original = f.handler._addToNodeFs
    const descriptor = Object.getOwnPropertyDescriptor(f.handler, names[1])
    f.results[names[0]] = pending.promise
    using observer = observeSubscription(f.watcher, directory, f.abort.signal)
    f.handler._addToNodeFs(directory)
    const error = new DOMException(directory, 'AbortError')
    const replacement = () => undefined
    const interrupt = {
      abort: () => f.abort.abort(error),
      error: () => f.watcher.emit('error', error),
      dispose: () => observer.dispose(),
      replacement: () => {
        f.handler._addToNodeFs = replacement
        observer.dispose()
      },
    }
    // When interrupted, then waiters settle without joining the hung promise or losing ownership.
    interrupt[action]()
    await expect(observer.ready).rejects.toBeInstanceOf(DOMException)
    observer.dispose()
    expect(Object.getOwnPropertyDescriptor(f.handler, names[1])).toEqual(
      descriptor,
    )
    expect(f.handler._addToNodeFs).toBe(
      action === 'replacement' ? replacement : original,
    )
    expect(f.watcher.listenerCount('error')).toBe(0)
    pending.resolve(false)
    await pending.promise
  },
)

it.each(
  names.flatMap((name) =>
    ['throw', 'reject', 'shape', 'fulfill'].map((mode) => ({ name, mode })),
  ),
)('preserves $name errors on $mode', async ({ name, mode }) => {
  // Given original exceptions, rejections or unsupported results at the private adapter boundary.
  const f = fixture()
  const error = new DOMException(directory, 'NotSupportedError')
  const pending = Promise.withResolvers<unknown>()
  const values: Record<string, unknown> = {
    throw: error,
    reject: pending.promise,
    shape: 42,
    fulfill: pending.promise,
  }
  f.results[name] = values[mode]
  using observer = observeSubscription(f.watcher, directory, f.abort.signal)
  // When the original operation fails, then its error/return identity survives without a thrown sidechain.
  if (mode === 'throw') expect(() => f.handler[name](directory)).toThrow(error)
  else {
    expect(f.handler[name](directory)).toBe(values[mode])
    if (mode === 'reject') {
      pending.reject(error)
      await expect(pending.promise).rejects.toBe(error)
    }
    if (mode === 'fulfill') {
      pending.resolve('unsupported')
      await pending.promise
    }
  }
  await expect(observer.ready).rejects.toBeInstanceOf(DOMException)
})

it.each(['backend', 'method'] as const)(
  'rejects unsupported %s before installing wrappers',
  (part) => {
    // Given an unsupported backend or a nonconfigurable later method.
    const f = fixture()
    if (part === 'backend') f.watcher.options.usePolling = true
    else Object.defineProperty(f.handler, names[2], { configurable: false })
    const original = f.handler._addToNodeFs
    // When installation fails, then even the earlier method has no partial wrapper.
    expect(() =>
      observeSubscription(f.watcher, directory, f.abort.signal),
    ).toThrow(DOMException)
    expect(f.handler._addToNodeFs).toBe(original)
  },
)

it('settles immediately when the supplied signal was already aborted', async () => {
  // Given cancellation before installation.
  const f = fixture()
  f.abort.abort()
  // When the observer is installed, then it does not strand its readiness promise.
  using observer = observeSubscription(f.watcher, directory, f.abort.signal)
  await expect(observer.ready).rejects.toBe(f.abort.signal.reason)
})
