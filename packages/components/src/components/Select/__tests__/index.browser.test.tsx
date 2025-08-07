import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { Select } from '..'

describe('Select', () => {
  it('should render', () => {
    const { container } = render(<Select />)
    expect(container).toMatchSnapshot()
  })
})
