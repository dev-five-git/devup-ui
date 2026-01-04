import * as reactModule from '@devup-ui/react'
import { afterAll, beforeAll, describe, expect, it, spyOn } from 'bun:test'

let globalCssSpy: ReturnType<typeof spyOn>

beforeAll(() => {
  globalCssSpy = spyOn(reactModule, 'globalCss').mockReturnValue(undefined)
})

afterAll(() => {
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
