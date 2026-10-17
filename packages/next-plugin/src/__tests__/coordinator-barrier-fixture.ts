import { IncomingMessage, Server } from 'node:http'

import { spyOn } from 'bun:test'

import { type CoordinatorOptions, startCoordinator } from '../coordinator'
import { createTestApp } from './coordinator-app'

export function barrier() {
  const app = createTestApp()
  const pending = Promise.withResolvers<CoordinatorOptions>()
  const entered = Promise.withResolvers<AbortSignal>()
  const handle = startCoordinator({
    projectRoot: app.root,
    identity: app.identity,
    coordinatorPortFile: app.portFile,
    prepare(signal) {
      entered.resolve(signal)
      return pending.promise
    },
  })
  return { app, pending, entered: entered.promise, handle }
}

export function acceptedRequests(count: number) {
  const accepted = Promise.withResolvers<void>()
  const emit = Server.prototype.emit
  let received = 0
  const spy = spyOn(Server.prototype, 'emit').mockImplementation(function (
    this: Server,
    event: string | symbol,
    ...args: unknown[]
  ) {
    const result: boolean = Reflect.apply(emit, this, [event, ...args])
    const req = args[0]
    if (
      event === 'request' &&
      req instanceof IncomingMessage &&
      (req.url?.startsWith('/css') || req.url === '/extract')
    ) {
      received += 1
      if (received === count) accepted.resolve()
    }
    return result
  })
  return { accepted: accepted.promise, restore: () => spy.mockRestore() }
}
