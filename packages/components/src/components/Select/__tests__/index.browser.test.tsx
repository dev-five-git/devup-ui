import { fireEvent, render } from '@testing-library/react'
import React from 'react'
import { describe, expect, it } from 'vitest'

import { Select, SelectContainer, SelectOption, SelectTrigger } from '..'
import { Default } from '../Default'

describe('Select', () => {
  it('should render', () => {
    const { container } = render(<Default />)
    expect(container).toMatchSnapshot()
  })

  it('should throw error when used outside of Select context', () => {
    expect(() => {
      render(<SelectOption />)
    }).toThrow()
  })

  it('should close select when clicking outside', () => {
    const { container } = render(
      <div data-testid="container">
        <Default />
      </div>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    const containerElement = container.querySelector(
      '[data-testid="container"]',
    )
    fireEvent.click(selectToggle!)
    expect(selectToggle).toHaveAttribute('aria-expanded', 'true')
    fireEvent.click(containerElement!)
    expect(selectToggle).toHaveAttribute('aria-expanded', 'false')
  })

  it('should call onOpenChange function when it is provided', () => {
    const onOpenChange = vi.fn()
    const { container } = render(
      <Default onOpenChange={onOpenChange} type="radio" />,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    expect(onOpenChange).toHaveBeenCalledWith(true)
  })

  it('should call onValueChange function when it is provided', () => {
    const onValueChange = vi.fn()
    const { container } = render(
      <Default onValueChange={onValueChange} type="radio" />,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    fireEvent.click(option2!)
    expect(onValueChange).toHaveBeenCalledWith('Option 2')
  })

  it('should do nothing when onValueChange is not provided and type is default', () => {
    const { container } = render(<Default type="default" />)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    fireEvent.click(option2!)
    fireEvent.click(selectToggle!)
    const option2_2 = container.querySelector('[data-value="Option 2"]')
    expect(option2_2?.querySelector('svg')).toBeNull()
  })

  it('should select option when type is radio and the option should have a check', () => {
    const { container } = render(<Default type="radio" />)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    fireEvent.click(option2!)
    fireEvent.click(selectToggle!)
    const option2_2 = container.querySelector('[data-value="Option 2"]')
    expect(option2_2?.querySelector('svg')).toBeInTheDocument()
  })

  it('should have multiple check marks when type is checkbox and multiple options are selected', () => {
    const { container } = render(<Default type="checkbox" />)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    const option3 = container.querySelector('[data-value="Option 3"]')
    expect(option2).toBeInTheDocument()
    expect(option3).toBeInTheDocument()
    fireEvent.click(option2!)
    fireEvent.click(option3!)
    expect(option2?.querySelector('svg')).toBeInTheDocument()
    expect(option3?.querySelector('svg')).toBeInTheDocument()
  })

  it('should not have a check mark when type is checkbox and the option is not selected', () => {
    const { container } = render(<Default type="checkbox" />)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    fireEvent.click(option2!)
    fireEvent.click(option2!)
    expect(option2?.querySelector('svg')).toBeNull()
  })

  it('should call onClick function when it is provided to SelectOption', () => {
    const onClick = vi.fn()
    const { container } = render(
      <Select>
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer>
          <SelectOption onClick={onClick} value="Option 1">
            Option 1
          </SelectOption>
          <SelectOption onClick={onClick} value="Option 2">
            Option 2
          </SelectOption>
        </SelectContainer>
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    fireEvent.click(option2!)
    expect(onClick).toHaveBeenCalledWith('Option 2', expect.any(Object))
  })

  it('should have a check mark when type is radio and defaultValue is provided', () => {
    const { container } = render(
      <Default defaultValue="Option 2" type="radio" />,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    expect(option2?.querySelector('svg')).toBeInTheDocument()
  })

  it('should not have a check mark when type is radio and defaultValue is not provided', () => {
    const { container } = render(<Default type="radio" />)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const selectContainer = container.querySelector(
      '[aria-label="Select container"]',
    )
    expect(selectContainer).toBeInTheDocument()
    expect(selectContainer?.querySelectorAll('svg')).toHaveLength(1)
  })

  it('should have 10px gap in an option when type is checkbox', () => {
    const { container } = render(<Default type="checkbox" />)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toHaveClass('gap-0-10px--1')
  })

  it('should have 6px gap in an option when type is radio', () => {
    const { container } = render(<Default type="radio" />)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toHaveClass('gap-0-6px--1')
  })

  it('should have 0 gap in an option when type is default', () => {
    const { container } = render(<Default type="default" />)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toHaveClass('gap-0-0--1')
  })

  it('should have undefined gap when type is not right', () => {
    // @ts-expect-error - test for wrong type
    const { container } = render(<Default type="no-type" />)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).not.toHaveClass('gap-0-0--1')
  })

  it('should handle ref.current being null by mocking useRef', () => {
    const useRefSpy = vi
      .spyOn(React, 'useRef')
      .mockReturnValueOnce({ current: null })

    const { container } = render(<Select>Select</Select>)

    // The component should render without errors even with null ref
    expect(container).toBeInTheDocument()

    // Trigger a click outside to test the useEffect logic with null ref
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    expect(selectToggle).toHaveAttribute('aria-expanded', 'true')

    // Click outside - this should trigger the useEffect but return early due to null ref
    fireEvent.click(document.body)

    // The select should still be open because the useEffect returned early
    expect(useRefSpy).toHaveBeenCalledWith({ current: null })
  })
})
