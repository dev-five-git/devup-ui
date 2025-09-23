import { act, render } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { Checkbox } from '../index'

describe('Checkbox', () => {
  it('should render basic checkbox', () => {
    const { container } = render(<Checkbox>Test Checkbox</Checkbox>)
    expect(container).toMatchSnapshot()
  })

  it('should render checked checkbox', () => {
    const { container } = render(<Checkbox checked>Test Checkbox</Checkbox>)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('input')).toBeChecked()
  })

  it('should render disabled checkbox', () => {
    const { container } = render(<Checkbox disabled>Test Checkbox</Checkbox>)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('input')).toBeDisabled()
  })

  it('should render disabled and checked checkbox', () => {
    const { container } = render(
      <Checkbox checked disabled>
        Test Checkbox
      </Checkbox>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('input')).toBeChecked()
    expect(container.querySelector('input')).toBeDisabled()
  })

  it('should render checkbox with custom child', () => {
    const { container } = render(
      <Checkbox>
        <div>Custom Child</div>
      </Checkbox>,
    )
    expect(container).toMatchSnapshot()
  })

  // onChange 로직 테스트
  it('should call onChange with true when checkbox is clicked and unchecked', async () => {
    const onChange = vi.fn()
    const { container } = render(
      <Checkbox checked={false} onChange={onChange}>
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
      <Checkbox checked={true} onChange={onChange}>
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
      <Checkbox disabled onChange={onChange}>
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
    const { container } = render(<Checkbox>Test Checkbox</Checkbox>)

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
      <Checkbox checked={false} disabled onChange={onChange}>
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
      <Checkbox checked={false} onChange={onChange}>
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
      <Checkbox disabled onChange={onChange}>
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
      <Checkbox checked={false} onChange={onChange}>
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
    const { container } = render(<Checkbox>Test Checkbox</Checkbox>)

    const input = container.querySelector('input')

    expect(input).toHaveAttribute('type', 'checkbox')
  })

  it('should display CheckIcon when checked', () => {
    const { container } = render(<Checkbox checked>Test Checkbox</Checkbox>)

    const checkIcon = container.querySelector('svg')
    expect(checkIcon).toBeInTheDocument()
  })

  it('should not display CheckIcon when unchecked', () => {
    const { container } = render(
      <Checkbox checked={false}>Test Checkbox</Checkbox>,
    )

    const checkIcon = container.querySelector('svg')
    expect(checkIcon).not.toBeInTheDocument()
  })

  it('should pass through additional props to input element', () => {
    const { container } = render(
      <Checkbox
        data-testid="custom-checkbox"
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

  // colors prop 테스트
  it('should render with custom colors', () => {
    const customColors = {
      primary: '#ff0000',
      border: '#00ff00',
      text: '#0000ff',
      inputBg: '#ffff00',
    }

    const { container } = render(
      <Checkbox colors={customColors}>Test Checkbox</Checkbox>,
    )

    expect(container).toMatchSnapshot()
  })

  it('should render with partial custom colors', () => {
    const partialColors = {
      primary: '#ff0000',
      text: '#0000ff',
    }

    const { container } = render(
      <Checkbox colors={partialColors}>Test Checkbox</Checkbox>,
    )

    expect(container).toMatchSnapshot()
  })

  it('should render checked checkbox with custom colors', () => {
    const customColors = {
      primary: '#ff0000',
      border: '#00ff00',
      text: '#0000ff',
      inputBg: '#ffff00',
      checkIcon: '#000000',
    }

    const { container } = render(
      <Checkbox checked colors={customColors}>
        Test Checkbox
      </Checkbox>,
    )

    expect(container).toMatchSnapshot()
  })

  it('should render disabled checkbox with custom colors', () => {
    const customColors = {
      primary: '#ff0000',
      border: '#00ff00',
      text: '#0000ff',
      inputBg: '#ffff00',
      checkIcon: '#000000',
    }

    const { container } = render(
      <Checkbox colors={customColors} disabled>
        Test Checkbox
      </Checkbox>,
    )

    expect(container).toMatchSnapshot()
  })

  it('should apply primary color to CSS variables', () => {
    const customColors = {
      primary: '#red-custom',
    }

    const { container } = render(
      <Checkbox colors={customColors}>Test Checkbox</Checkbox>,
    )

    const input = container.querySelector('input')
    expect(input).toHaveStyle({
      '--primary': '#red-custom',
    })
  })

  it('should apply border color to CSS variables', () => {
    const customColors = {
      border: '#border-custom',
    }

    const { container } = render(
      <Checkbox colors={customColors}>Test Checkbox</Checkbox>,
    )

    const input = container.querySelector('input')
    expect(input).toHaveStyle({
      '--border': '#border-custom',
    })
  })

  it('should apply text color to CSS variables', () => {
    const customColors = {
      text: '#text-custom',
    }

    const { container } = render(
      <Checkbox colors={customColors}>Test Checkbox</Checkbox>,
    )

    const input = container.querySelector('input')
    expect(input).toHaveStyle({
      '--text': '#text-custom',
    })
  })

  it('should apply inputBg color to CSS variables', () => {
    const customColors = {
      inputBg: '#inputBg-custom',
    }

    const { container } = render(
      <Checkbox colors={customColors}>Test Checkbox</Checkbox>,
    )

    const input = container.querySelector('input')
    expect(input).toHaveStyle({
      '--inputBg': '#inputBg-custom',
    })
  })

  it('should apply checkIcon color to CSS variables', () => {
    const customColors = {
      checkIcon: '#checkIcon-custom',
    }

    const { container } = render(
      <Checkbox colors={customColors}>Test Checkbox</Checkbox>,
    )

    const input = container.querySelector('input')
    expect(input).toHaveStyle({
      '--checkIcon': '#checkIcon-custom',
    })
  })

  it('should apply all custom colors to CSS variables', () => {
    const customColors = {
      primary: '#primary-custom',
      border: '#border-custom',
      text: '#text-custom',
      inputBg: '#inputBg-custom',
      checkIcon: '#checkIcon-custom',
    }

    const { container } = render(
      <Checkbox colors={customColors}>Test Checkbox</Checkbox>,
    )

    const input = container.querySelector('input')
    expect(input).toHaveStyle({
      '--primary': '#primary-custom',
      '--border': '#border-custom',
      '--text': '#text-custom',
      '--inputBg': '#inputBg-custom',
      '--checkIcon': '#checkIcon-custom',
    })
  })

  it('should not apply CSS variables when colors prop is not provided', () => {
    const { container } = render(<Checkbox>Test Checkbox</Checkbox>)

    const input = container.querySelector('input')
    // CSS 변수가 undefined로 설정되지 않아야 함
    expect(input?.style.getPropertyValue('--primary')).toBe('')
    expect(input?.style.getPropertyValue('--border')).toBe('')
    expect(input?.style.getPropertyValue('--text')).toBe('')
    expect(input?.style.getPropertyValue('--inputBg')).toBe('')
    expect(input?.style.getPropertyValue('--checkIcon')).toBe('')
  })

  it('should work properly with onChange when colors are applied', async () => {
    const onChange = vi.fn()
    const customColors = {
      primary: '#ff0000',
      border: '#00ff00',
      text: '#0000ff',
      inputBg: '#ffff00',
      checkIcon: '#000000',
    }

    const { container } = render(
      <Checkbox checked={false} colors={customColors} onChange={onChange}>
        Test Checkbox
      </Checkbox>,
    )

    const input = container.querySelector('input')

    await act(async () => {
      await userEvent.click(input!)
    })

    expect(onChange).toHaveBeenCalledWith(true)
    expect(onChange).toHaveBeenCalledTimes(1)
  })
})
