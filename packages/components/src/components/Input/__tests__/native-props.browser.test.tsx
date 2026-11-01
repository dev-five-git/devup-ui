import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { Input } from '..'

describe('Input native props', () => {
  it('should render a read-only native input when readOnly is true', () => {
    const { container } = render(<Input readOnly />)

    expect(container.querySelector('input')?.getAttribute('readonly')).toBe('')
  })

  it('should hide the clear button when readOnly and a value are provided', () => {
    const { container } = render(<Input defaultValue="test" readOnly />)

    expect(
      container.querySelector('[aria-label="clear-button"]'),
    ).not.toBeInTheDocument()
  })

  it('should show the clear button when a value is provided without readOnly', () => {
    const { container } = render(<Input defaultValue="test" />)

    expect(
      container.querySelector('[aria-label="clear-button"]'),
    ).toBeInTheDocument()
    expect(container.querySelector('input')).not.toHaveAttribute('readonly')
  })
})
