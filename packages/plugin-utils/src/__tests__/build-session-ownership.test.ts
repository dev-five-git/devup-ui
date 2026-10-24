import { describe, expect, it } from 'bun:test'

import { beginBuild, getBuildOwnership } from '../build-session'

describe('shared build ownership', () => {
  it('shares cold ownership across independently loaded module copies', async () => {
    // Given: shadow only the globalThis alias, never the live ownership slot.
    const actualGlobal = globalThis
    const descriptor = Object.getOwnPropertyDescriptor(
      actualGlobal,
      'globalThis',
    )
    if (!descriptor) throw new TypeError('globalThis descriptor is required')
    const liveOwnership = actualGlobal.__devupBuildOwnership
    const liveSnapshot = getBuildOwnership()
    const shadow: Pick<typeof globalThis, '__devupBuildOwnership'> = {
      __devupBuildOwnership: undefined,
    }
    Object.setPrototypeOf(shadow, actualGlobal)
    Object.defineProperty(shadow, 'globalThis', { value: shadow })
    let sheet = 'stale'
    const engine = {
      resetBuildState() {
        sheet = ''
      },
    }
    let endA: (() => void) | undefined
    let endB: (() => void) | undefined
    try {
      let a: typeof import('../build-session')
      let b: typeof import('../build-session')
      Object.defineProperty(actualGlobal, 'globalThis', {
        ...descriptor,
        value: shadow,
      })
      try {
        a = await import(`${import.meta.dir}/../build-session.ts?cold-owner-a`)
        expect(a.getBuildOwnership()).toEqual({ running: 0, generation: 0 })
        endA = a.beginBuild(engine)
        expect(sheet).toBe('')
        sheet = 'active'
        const ownership = shadow.__devupBuildOwnership
        b = await import(`${import.meta.dir}/../build-session.ts?cold-owner-b`)
        expect(b).not.toBe(a)
        expect(shadow.__devupBuildOwnership).toBe(ownership)
      } finally {
        Object.defineProperty(actualGlobal, 'globalThis', descriptor)
      }
      // When: a second evaluated copy joins the first copy's active build.
      endB = b.beginBuild(engine)
      // Then: overlapping owners share state without wiping the active engine.
      expect(sheet).toBe('active')
      expect(a.getBuildOwnership()).toEqual({ running: 2, generation: 2 })
      expect(b.getBuildOwnership()).toEqual({ running: 2, generation: 2 })
      endA()
      endA()
      expect(b.getBuildOwnership()).toEqual({ running: 1, generation: 3 })
      endB()
      endB()
      expect(a.getBuildOwnership()).toEqual({ running: 0, generation: 4 })
    } finally {
      Object.defineProperty(actualGlobal, 'globalThis', descriptor)
      endB?.()
      endA?.()
      expect(actualGlobal.__devupBuildOwnership).toBe(liveOwnership)
      expect(getBuildOwnership()).toEqual(liveSnapshot)
      expect(
        Object.getOwnPropertyDescriptor(actualGlobal, 'globalThis'),
      ).toEqual(descriptor)
    }
  })

  it('preserves active state when another owner starts and advances transitions', () => {
    // Given
    let sheet = 'stale'
    const engine = {
      resetBuildState() {
        sheet = ''
      },
    }
    const initial = getBuildOwnership()
    const a = beginBuild(engine)
    expect(sheet).toBe(initial.running === 0 ? '' : 'stale')
    sheet = 'active'
    const before = getBuildOwnership()
    // When
    const b = beginBuild(engine)
    // Then
    try {
      expect(sheet).toBe('active')
      expect(getBuildOwnership()).toEqual({
        running: before.running + 1,
        generation: before.generation + 1,
      })
    } finally {
      b()
      a()
    }
    expect(getBuildOwnership().running).toBe(initial.running)
    expect(getBuildOwnership().generation).toBe(initial.generation + 4)
    a()
    expect(getBuildOwnership().generation).toBe(initial.generation + 4)
  })

  it('remains compatible when an older facade has no reset export', () => {
    // Given
    const initial = getBuildOwnership()
    // When
    const end = beginBuild({})
    end()
    // Then
    expect(getBuildOwnership()).toEqual({
      running: initial.running,
      generation: initial.generation + 2,
    })
  })
})
