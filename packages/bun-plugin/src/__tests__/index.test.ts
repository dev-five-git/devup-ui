import { describe, expect, it } from 'bun:test'

import * as exports from '../index'

describe('index exports', () => {
  it('should export getDevupDefaultTheme', () => {
    expect(exports.getDevupDefaultTheme).toBeInstanceOf(Function)
  })

  it('should export getDevupDefine', () => {
    expect(exports.getDevupDefine).toBeInstanceOf(Function)
  })
})
