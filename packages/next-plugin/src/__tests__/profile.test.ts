import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import {
  elapsedMs,
  isProfileEnabled,
  profileStart,
  reportProfile,
} from '../profile'

let originalEnv: NodeJS.ProcessEnv
let originalEnvValues: NodeJS.ProcessEnv

beforeEach(() => {
  originalEnv = process.env
  originalEnvValues = { ...originalEnv }
})

afterEach(() => {
  for (const key of Object.keys(originalEnv)) {
    if (!Object.hasOwn(originalEnvValues, key)) delete originalEnv[key]
  }
  Object.assign(originalEnv, originalEnvValues)
  process.env = originalEnv
})

describe('profile', () => {
  it('is disabled unless explicitly enabled', () => {
    delete process.env.DEVUP_UI_PROFILE
    const consoleSpy = spyOn(console, 'info').mockImplementation(() => {})

    try {
      expect(isProfileEnabled()).toBe(false)
      reportProfile('next.prewarm')
      expect(consoleSpy).not.toHaveBeenCalled()
    } finally {
      consoleSpy.mockRestore()
    }
  })

  it('reports structured measurements when enabled', () => {
    process.env.DEVUP_UI_PROFILE = '1'
    const consoleSpy = spyOn(console, 'info').mockImplementation(() => {})

    try {
      reportProfile('next.prewarm', { durationMs: 12.34, files: 2 })

      expect(consoleSpy).toHaveBeenCalledWith(
        '[devup-ui:profile] {"phase":"next.prewarm","durationMs":12.34,"files":2}',
      )
    } finally {
      consoleSpy.mockRestore()
    }
  })

  it('returns a non-negative elapsed duration', () => {
    expect(elapsedMs(performance.now())).toBeGreaterThanOrEqual(0)
  })

  it('does not start a timer while disabled', () => {
    delete process.env.DEVUP_UI_PROFILE

    expect(profileStart()).toBeUndefined()
    expect(elapsedMs(undefined)).toBeUndefined()
  })

  it('starts a timer when enabled', () => {
    process.env.DEVUP_UI_PROFILE = '1'

    expect(profileStart()).toBeTypeOf('number')
  })
})
