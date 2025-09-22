import { act, render } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { Checkbox } from '../index'

describe('Checkbox', () => {
  it('should render basic checkbox', () => {
    const { container } = render(
      <Checkbox label="test-checkbox">Test Checkbox</Checkbox>,
    )
    expect(container).toMatchSnapshot()
  })

  it('should render checked checkbox', () => {
    const { container } = render(
      <Checkbox checked label="test-checkbox">
        Test Checkbox
      </Checkbox>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('input')).toBeChecked()
  })

  it('should render disabled checkbox', () => {
    const { container } = render(
      <Checkbox disabled label="test-checkbox">
        Test Checkbox
      </Checkbox>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('input')).toBeDisabled()
  })

  it('should render disabled and checked checkbox', () => {
    const { container } = render(
      <Checkbox checked disabled label="test-checkbox">
        Test Checkbox
      </Checkbox>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('input')).toBeChecked()
    expect(container.querySelector('input')).toBeDisabled()
  })

  it('should render checkbox with custom child', () => {
    const { container } = render(
      <Checkbox label="test-checkbox">
        <div>Custom Child</div>
      </Checkbox>,
    )
    expect(container).toMatchSnapshot()
  })

  // onChange 로직 테스트
  it('should call onChange with true when checkbox is clicked and unchecked', async () => {
    const onChange = vi.fn()
    const { container } = render(
      <Checkbox checked={false} label="test-checkbox" onChange={onChange}>
        Test Checkbox
      </Checkbox>,
    )

    const input = container.querySelector('input')
    expect(input).toBeInTheDocument()

    await act(async () => {
      await userEvent.click(input!)
    })

    expect(onChange).toHaveBeenCalledWith(true)
    expect(onChange).toHaveBeenCalledTimes(1)
  })

  it('should call onChange with false when checkbox is clicked and checked', async () => {
    const onChange = vi.fn()
    const { container } = render(
      <Checkbox checked={true} label="test-checkbox" onChange={onChange}>
        Test Checkbox
      </Checkbox>,
    )

    const input = container.querySelector('input')

    await act(async () => {
      await userEvent.click(input!)
    })

    expect(onChange).toHaveBeenCalledWith(false)
    expect(onChange).toHaveBeenCalledTimes(1)
  })

  it('should not call onChange when disabled is true', async () => {
    const onChange = vi.fn()
    const { container } = render(
      <Checkbox disabled label="test-checkbox" onChange={onChange}>
        Test Checkbox
      </Checkbox>,
    )

    const input = container.querySelector('input')

    await act(async () => {
      await userEvent.click(input!)
    })

    expect(onChange).not.toHaveBeenCalled()
  })

  it('should not call onChange when onChange prop is not provided', async () => {
    const { container } = render(
      <Checkbox label="test-checkbox">Test Checkbox</Checkbox>,
    )

    const input = container.querySelector('input')

    // Should not throw error when clicking without onChange
    await act(async () => {
      await userEvent.click(input!)
    })

    // Test passes if no error is thrown
    expect(true).toBe(true)
  })

  it('should not call onChange when both disabled and onChange are provided', async () => {
    const onChange = vi.fn()
    const { container } = render(
      <Checkbox
        checked={false}
        disabled
        label="test-checkbox"
        onChange={onChange}
      >
        Test Checkbox
      </Checkbox>,
    )

    const input = container.querySelector('input')

    await act(async () => {
      await userEvent.click(input!)
    })

    expect(onChange).not.toHaveBeenCalled()
  })

  it('should handle label click and trigger onChange', async () => {
    const onChange = vi.fn()
    const { container } = render(
      <Checkbox checked={false} label="test-checkbox" onChange={onChange}>
        Test Checkbox
      </Checkbox>,
    )

    const label = container.querySelector('label')
    expect(label).toBeInTheDocument()

    await act(async () => {
      await userEvent.click(label!)
    })

    expect(onChange).toHaveBeenCalledWith(true)
  })

  it('should not trigger onChange on label click when disabled', async () => {
    const onChange = vi.fn()
    const { container } = render(
      <Checkbox disabled label="test-checkbox" onChange={onChange}>
        Test Checkbox
      </Checkbox>,
    )

    const label = container.querySelector('label')

    await act(async () => {
      await userEvent.click(label!)
    })

    expect(onChange).not.toHaveBeenCalled()
  })

  it('should pass correct event target checked value to onChange', async () => {
    const onChange = vi.fn()
    const { container } = render(
      <Checkbox checked={false} label="test-checkbox" onChange={onChange}>
        Test Checkbox
      </Checkbox>,
    )

    const input = container.querySelector('input') as HTMLInputElement

    await act(async () => {
      await userEvent.click(input!)
    })

    expect(onChange).toHaveBeenCalledWith(true)
    expect(input.checked).toBe(false)
  })

  it('should have proper accessibility attributes', () => {
    const { container } = render(
      <Checkbox label="test-checkbox">Test Checkbox</Checkbox>,
    )

    const input = container.querySelector('input')
    const label = container.querySelector('label')

    expect(input).toHaveAttribute('id', 'test-checkbox')
    expect(input).toHaveAttribute('type', 'checkbox')
    expect(label).toHaveAttribute('for', 'test-checkbox')
  })

  it('should display CheckIcon when checked', () => {
    const { container } = render(
      <Checkbox checked label="test-checkbox">
        Test Checkbox
      </Checkbox>,
    )

    const checkIcon = container.querySelector('svg')
    expect(checkIcon).toBeInTheDocument()
  })

  it('should not display CheckIcon when unchecked', () => {
    const { container } = render(
      <Checkbox checked={false} label="test-checkbox">
        Test Checkbox
      </Checkbox>,
    )

    const checkIcon = container.querySelector('svg')
    expect(checkIcon).not.toBeInTheDocument()
  })

  it('should pass through additional props to input element', () => {
    const { container } = render(
      <Checkbox
        data-testid="custom-checkbox"
        label="test-checkbox"
        name="test-name"
        value="test-value"
      >
        Test Checkbox
      </Checkbox>,
    )

    const input = container.querySelector('input')
    expect(input).toHaveAttribute('data-testid', 'custom-checkbox')
    expect(input).toHaveAttribute('name', 'test-name')
    expect(input).toHaveAttribute('value', 'test-value')
  })
})
