import { describe, expect, it } from 'bun:test'

import { beginBuild, getBuildOwnership } from '../build-session'

describe('shared build ownership', () => {
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
