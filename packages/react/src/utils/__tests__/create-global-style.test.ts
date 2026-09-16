import { describe, expect, it } from 'bun:test'

import { createGlobalStyle } from '../create-global-style'

describe('createGlobalStyle', () => {
  it('cannot run on the runtime', () => {
    expect(() => createGlobalStyle`body { margin: 0; }`).toThrowError(
      'Cannot run on the runtime',
    )
    expect(() => createGlobalStyle({ body: { margin: 0 } })).toThrowError(
      'Cannot run on the runtime',
    )
  })
})
