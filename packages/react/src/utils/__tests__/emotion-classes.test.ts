import { describe, expect, it } from 'bun:test'

import { cx, merge } from '../emotion-classes'

describe('emotion class composition', () => {
  it('cx cannot run on the runtime', () => {
    expect(() => cx('a', { b: true }, ['c'])).toThrowError(
      'Cannot run on the runtime',
    )
  })

  it('merge cannot run on the runtime', () => {
    expect(() => merge('a b')).toThrowError('Cannot run on the runtime')
  })
})
