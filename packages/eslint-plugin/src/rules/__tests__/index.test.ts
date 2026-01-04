import { describe, expect, it } from 'bun:test'

import * as index from '../index'

describe('export index', () => {
  it('export', () => {
    expect({ ...index }).toEqual({
      noUselessTailingNulls: expect.any(Object),
      cssUtilsLiteralOnly: expect.any(Object),
      noDuplicateValue: expect.any(Object),
      noUselessResponsive: expect.any(Object),
      styleOrderRange: expect.any(Object),
    })
  })
})
