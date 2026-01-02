import * as reactModule from '@devup-ui/react'
import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

let globalCssSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  globalCssSpy = spyOn(reactModule, 'globalCss').mockReturnValue(undefined)
})

afterEach(() => {
  globalCssSpy.mockRestore()
})

describe('reset-css', () => {
  it('should be defined', async () => {
    // Dynamic import to ensure spy is in place
    const { resetCss } = await import('../index')
    expect(resetCss).toBeInstanceOf(Function)
    expect(resetCss()).toBeUndefined()
  })
})
