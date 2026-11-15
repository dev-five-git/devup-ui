import { expect, it } from 'bun:test'

import { getBuildOwnership } from '../../../plugin-utils/src/build-session'

it('shares cold ownership counters when a separately evaluated facade joins a build', async () => {
  // Given an isolated global alias, leaving the live lane ownership untouched.
  const actualGlobal = globalThis
  const descriptor = Object.getOwnPropertyDescriptor(actualGlobal, 'globalThis')
  if (descriptor === undefined)
    throw new TypeError('globalThis descriptor required')
  const liveOwnership = actualGlobal.__devupBuildOwnership
  const liveSnapshot = getBuildOwnership()
  const shadow: Pick<typeof globalThis, '__devupBuildOwnership'> = {
    __devupBuildOwnership: undefined,
  }
  Object.setPrototypeOf(shadow, actualGlobal)
  Object.defineProperty(shadow, 'globalThis', { value: shadow })
  let release: (() => void) | undefined
  try {
    Object.defineProperty(actualGlobal, 'globalThis', {
      ...descriptor,
      value: shadow,
    })
    const facade = await import(
      `${import.meta.dir}/../../../plugin-utils/src/build-session.ts?vite-cold-ownership`
    )
    expect(facade.getBuildOwnership()).toEqual({ running: 0, generation: 0 })
    const before = facade.getBuildOwnership()
    // When admission starts a build through that actual facade.
    release = facade.beginBuild({})
    // Then returned snapshots are detached and reflect shared admission transitions.
    expect(before).toEqual({ running: 0, generation: 0 })
    expect(facade.getBuildOwnership()).toEqual({ running: 1, generation: 1 })
    release()
    release()
    expect(facade.getBuildOwnership()).toEqual({ running: 0, generation: 2 })
  } finally {
    release?.()
    Object.defineProperty(actualGlobal, 'globalThis', descriptor)
  }
  expect(actualGlobal.__devupBuildOwnership).toBe(liveOwnership)
  expect(getBuildOwnership()).toEqual(liveSnapshot)
})
