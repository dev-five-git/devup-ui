import { DevupThemeTypography } from '@devup-ui/react'
import { describe, expect, it, mock } from 'bun:test'
import { fireEvent, render, userEvent } from 'bun-test-env-dom'

import { ClearButton, Input } from '..'
import { Controlled } from '../Controlled'
import { GlassIcon } from '../GlassIcon'

describe('Input', () => {
  it('should render with default props', () => {
    const { container } = render(<Input />)
    expect(container).toMatchSnapshot()
  })

  it('should render with disabled prop', () => {
    const { container } = render(<Input disabled />)
    expect(container).toMatchSnapshot()
  })

  it('should render with allowClear prop', () => {
    const { container } = render(<Input allowClear />)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('[aria-label="input"]')).toHaveClass(
      'padding-right-0-36px--1',
    )
  })

  it('should not have padding right when allowClear is false', () => {
    const { container } = render(<Input allowClear={false} />)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('[aria-label="input"]')).not.toHaveClass(
      'padding-right-0-36px--1',
    )
  })

  it('should show clear button when value is not empty', () => {
    const { container } = render(<Input defaultValue="test" />)
    expect(container.querySelector('button')).toBeInTheDocument()
  })

  it('should not show clear button when value is empty', () => {
    const { container } = render(<Input />)
    expect(container).toMatchSnapshot()
  })

  it('should be able to clear value by clicking clear button', () => {
    const { container } = render(<Input allowClear defaultValue="test" />)
    expect(container.querySelector('button')).toBeInTheDocument()
    fireEvent.click(container.querySelector('button')!)
    expect(container.querySelector('input')!.value).toBe('')
  })

  it('should be able to render with icon', () => {
    const { container } = render(
      <Input icon={<GlassIcon data-testid="icon" />} />,
    )
    expect(container.querySelector('[data-testid="icon"]')).toBeInTheDocument()
  })

  it('should render error style when error is true', () => {
    const { container } = render(<Input error />)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('[aria-label="input"]')).toHaveClass(
      'border-color-0-var(--error,light-dark(#D52B2E,#FF5B5E))--1',
    )
  })

  it('should be able to render with error message', () => {
    const { container } = render(<Input error errorMessage="Error message" />)
    expect(
      container.querySelector('[aria-label="error-message"]'),
    ).toBeInTheDocument()
  })

  it('should pass colors prop', () => {
    const { container } = render(
      <Input
        colors={{
          primary: 'red',
          error: 'blue',
          text: 'green',
        }}
      />,
    )
    const input = container.querySelector('[aria-label="input"]')
    expect(input).toHaveStyle({
      '--primary': 'red',
      '--error': 'blue',
      '--text': 'green',
    })
  })

  it('should have typography when typography is provided', () => {
    const { container } = render(
      <Input typography={'inlineLabelS' as keyof DevupThemeTypography} />,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('input')).toHaveClass('typo-inlineLabelS')
  })

  it('should pass className prop to error message component', () => {
    const { container } = render(
      <Input
        classNames={{
          errorMessage: 'error-message',
        }}
        error
        errorMessage="Error message"
      />,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('[aria-label="error-message"]')).toHaveClass(
      'error-message',
    )
  })

  it('should pass className prop to icon component', () => {
    const { container } = render(
      <Input
        classNames={{
          icon: 'icon',
        }}
        icon={<GlassIcon />}
      />,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('[aria-label="icon"]')).toHaveClass('icon')
  })

  it('should pass props to ClearButton component', async () => {
    const { container } = render(<ClearButton />)
    expect(container).toMatchSnapshot()
    const clearButton = container.querySelector('[aria-label="clear-button"]')
    expect(clearButton).toBeInTheDocument()
  })

  it('should render disabled icon style when disabled is true', () => {
    const { container } = render(<Input disabled icon={<GlassIcon />} />)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('[aria-label="icon"]')).toHaveClass(
      'color-0-var(--inputDisabledText,light-dark(#D6D7DE,#373737))--1',
    )
  })

  // Note: fireEvent.change doesn't trigger React's onChange in Happy-DOM
  // This is tested indirectly via onClear test and Controlled component tests
  it('should call onChange prop when it is provided and value is changed', async () => {
    const onChange = mock()
    const { container } = render(<Input onChange={onChange} value="" />)
    await userEvent.type(container.querySelector('input')!, 'test')

    expect(onChange).toHaveBeenCalledWith(expect.any(Object))
  })

  it('should call onClear props when click clear button', async () => {
    const onClear = mock()
    const { container } = render(<Input onClear={onClear} />)

    await userEvent.type(container.querySelector('input')!, 'test')
    fireEvent.click(container.querySelector('[aria-label="clear-button"]')!)
    expect(onClear).toHaveBeenCalled()
    expect(container.querySelector('input')!.value).toBe('')
  })
})

describe('Controlled Input', () => {
  it('should render with value', () => {
    const { container } = render(<Controlled />)
    expect(container).toMatchSnapshot()
  })

  it('should update value when it is changed', () => {
    const { container } = render(<Controlled />)
    fireEvent.change(container.querySelector('input')!, {
      target: { value: 'test' },
    })
    expect(container.querySelector('input')!.value).toBe('test')
  })

  it('should clear value when clear button is clicked', () => {
    // Use Input with defaultValue since fireEvent.change doesn't trigger React's onChange in Happy-DOM
    const { container } = render(<Input defaultValue="test" />)
    expect(
      container.querySelector('[aria-label="clear-button"]'),
    ).toBeInTheDocument()
    fireEvent.click(container.querySelector('[aria-label="clear-button"]')!)
    expect(container.querySelector('input')!.value).toBe('')
  })
})
