import { describe, expect, it } from 'bun:test'

import recommended from '../recommended'

describe('recommended', () => {
  it('export recommended config', () => {
    expect(recommended).toMatchSnapshot()
  })
})
