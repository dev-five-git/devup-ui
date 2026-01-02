import * as wasm from '@devup-ui/wasm'
import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

let getDefaultThemeSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  getDefaultThemeSpy = spyOn(wasm, 'getDefaultTheme').mockReturnValue('default')
})

afterEach(() => {
  getDefaultThemeSpy.mockRestore()
})

describe('getDevupDefine', () => {
  it('should return define object with theme', async () => {
    getDefaultThemeSpy.mockReturnValue('dark')
    expect(getDefaultThemeSpy()).toBe('dark')
  })

  it('should return empty object when no theme', async () => {
    getDefaultThemeSpy.mockReturnValue(undefined)
    expect(getDefaultThemeSpy()).toBe(undefined)
  })
})
