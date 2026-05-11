import { describe, expect, it } from 'bun:test'

import { globalCss } from '../global-css'

describe('globalCss', () => {
  it('should return className', () => {
    expect(() => globalCss`virtual-css`).toThrowError(
      'Cannot run on the runtime',
    )
    expect(() =>
      globalCss('class name' as unknown as Parameters<typeof globalCss>[0]),
    ).toThrowError('Cannot run on the runtime')
    expect(() => globalCss()).toThrowError('Cannot run on the runtime')
  })
})
