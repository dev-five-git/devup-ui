import { describe, expect, it } from 'bun:test'

import * as index from '../index'

describe('export index', () => {
  it('export', () => {
    expect({ ...index }).toEqual({
      rules: expect.any(Object),
      configs: {
        recommended: expect.any(Object),
        recommendedTypeChecked: expect.any(Array),
      },
      default: {
        configs: {
          recommended: expect.any(Object),
          recommendedTypeChecked: expect.any(Array),
        },
      },
    })
  })
})
