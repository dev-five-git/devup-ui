import { expect, it } from 'bun:test'

import { createWebpackGenerationThread } from '../webpack-generation'

it('shares only the live public context when related compiler configs are constructed', () => {
  // Given
  const thread = createWebpackGenerationThread()
  const config = {}
  const server = thread({ config, dev: false, isServer: true })
  // When
  const client = thread({ config, dev: false, isServer: false })
  // Then
  expect(client.owner).toBe(server.owner)
  expect(server.complete).toBe(false)
  expect(client.complete).toBe(true)
})

it('mints a fresh generation when Next reuses its cached config after completion', () => {
  // Given
  const thread = createWebpackGenerationThread()
  const context = { config: {}, dev: false, isServer: false, buildId: 'fixed' }
  const first = thread(context)
  first.owner.acquire(first.complete)()
  // When
  const second = thread(context)
  // Then
  expect(second.owner).not.toBe(first.owner)
  expect(second.owner.disposed).toBe(false)
})

it('keeps independent wrapper calls apart even with identical public context and fixed buildId', () => {
  // Given
  const context = { config: {}, dev: false, isServer: true, buildId: 'fixed' }
  const first = createWebpackGenerationThread()(context)
  // When
  const second = createWebpackGenerationThread()(context)
  // Then
  expect(second.owner).not.toBe(first.owner)
})

it('does not pin a generation when the edge configuration remains unused', () => {
  // Given
  const thread = createWebpackGenerationThread()
  const config = {}
  thread({ config, dev: false, isServer: true })
  const client = thread({ config, dev: false, isServer: false })
  // When
  client.owner.acquire(client.complete)()
  // Then
  expect(thread({ config, dev: false, isServer: true }).owner).not.toBe(
    client.owner,
  )
})

it('closes dev ownership only when every actual watch participant closes', () => {
  // Given
  const thread = createWebpackGenerationThread()
  const config = {}
  const server = thread({ config, dev: true, isServer: true })
  const client = thread({ config, dev: true, isServer: false })
  const serverClose = server.owner.acquire(server.complete)
  client.owner.acquire(client.complete)()
  expect(server.owner.disposed).toBe(false)
  // When
  serverClose()
  // Then
  expect(thread({ config, dev: true, isServer: true }).owner).not.toBe(
    server.owner,
  )
})
