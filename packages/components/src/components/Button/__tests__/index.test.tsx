import { render } from '@testing-library/react'
import { expect } from 'vitest'

import { Button } from '../index'

describe('Button', () => {
  it('should render', () => {
    const { container } = render(<Button>Click me</Button>)
    expect(container).toMatchSnapshot()
  })

  it('should disable', () => {
    const { container } = render(<Button disabled>Click me</Button>)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveAttribute('disabled')
  })
})
