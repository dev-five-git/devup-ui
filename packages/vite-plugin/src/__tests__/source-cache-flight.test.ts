import { mkdirSync, rmSync } from 'node:fs'

import { expect, it, mock } from 'bun:test'

import {
  cacheFixture,
  captureBoundary,
  extractionObserver,
  rejectionBoundary,
  SOURCE,
  writeGate,
} from './source-cache-fixture'
it('shares one real extraction when identical callers overlap a real write', async () => {
  // Given
  using state = await cacheFixture()
  using extracts = extractionObserver()
  using write = writeGate(state.cssFile)
  const first = state.transform()
  let second: Promise<unknown> | undefined
  try {
    await write.entered
    using captured = captureBoundary()
    const addWatchFile = mock()
    // When: a second call reaches dependency registration before write release.
    second = state.plugin.transform.call({ addWatchFile }, SOURCE, state.id)
    await captured.reached
    expect(addWatchFile).toHaveBeenCalledWith(state.supplier)
    expect(extracts).toHaveBeenCalledTimes(1)
    write.release()
    // Then: both results and the later completed hit use the same extraction.
    expect(await second).toEqual(await first)
    await state.transform()
    expect(extracts).toHaveBeenCalledTimes(1)
    expect(
      write.spy.mock.calls.filter((args) => String(args[0]) === state.cssFile),
    ).toHaveLength(1)
  } finally {
    write.release()
    await Promise.allSettled([first, second])
  }
})

it('propagates a real filesystem write rejection and extracts again on retry', async () => {
  // Given: writing CSS onto a directory causes the actual platform write error.
  using state = await cacheFixture()
  mkdirSync(state.cssFile)
  using extracts = extractionObserver()
  using write = writeGate(state.cssFile)
  const first = state.transform()
  const rejected = rejectionBoundary(first)
  try {
    await write.entered
    // When: let that genuine write error reach the flight owner.
    rmSync(state.cssFile, { recursive: true })
    write.release()
    await expect(rejected()).rejects.toBeInstanceOf(Error)
    const retry = await state.transform()
    // Then: the rejected entry is not retained as failure or success.
    expect(retry).toBeDefined()
    expect(extracts).toHaveBeenCalledTimes(2)
    await state.transform()
    expect(extracts).toHaveBeenCalledTimes(2)
  } finally {
    write.release()
    await Promise.allSettled([first])
  }
})

it('keeps a newer same-ID flight when the older real write rejects', async () => {
  // Given: A's actual failed write is settled internally but held at its boundary.
  using state = await cacheFixture()
  mkdirSync(state.cssFile)
  using extracts = extractionObserver()
  using write = writeGate(state.cssFile)
  const first = state.transform()
  const rejected = rejectionBoundary(first)
  let newer: Promise<unknown> | undefined
  try {
    await write.entered
    rmSync(state.cssFile, { recursive: true })
    const b = SOURCE.replace('color={tone}', 'bg={tone}')
    using captured = captureBoundary()
    const addWatchFile = mock()
    // When: B creates its own same-ID flight while A's rejection is still held.
    newer = state.plugin.transform.call({ addWatchFile }, b, state.id)
    await captured.reached
    expect(addWatchFile).toHaveBeenCalledWith(state.supplier)
    expect(extracts).toHaveBeenCalledTimes(2)
    write.release()
    await expect(rejected()).rejects.toBeInstanceOf(Error)
    const result = await newer
    // Then: A may delete only A, not the newer B flight or physical observation.
    expect(await state.transform(b)).toEqual(result)
    expect(extracts).toHaveBeenCalledTimes(2)
  } finally {
    write.release()
    await Promise.allSettled([first, newer])
  }
})
