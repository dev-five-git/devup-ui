import { fireEvent, render } from '@testing-library/react'
import { DevupThemeTypography } from 'node_modules/@devup-ui/react/dist/types/typography'

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
    const { container } = render(<Input />)
    expect(container).toMatchSnapshot()
    fireEvent.change(container.querySelector('input')!, {
      target: { value: 'test' },
    })
    expect(container.querySelector('button')).toBeInTheDocument()
  })

  it('should not show clear button when value is empty', () => {
    const { container } = render(<Input />)
    expect(container).toMatchSnapshot()
  })

  it('should be able to clear value by clicking clear button', () => {
    const { container } = render(<Input allowClear />)
    fireEvent.change(container.querySelector('input')!, {
      target: { value: 'test' },
    })
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
    const { container } = render(<Input errorMessage="Error message" />)
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

  it('should call onChange prop when it is provided andvalue is changed', () => {
    const onChange = vi.fn()
    const { container } = render(<Input onChange={onChange} />)
    fireEvent.change(container.querySelector('input')!, {
      target: { value: 'test' },
    })
    expect(onChange).toHaveBeenCalledWith(expect.any(Object))
  })

  it('should call onClear props when click clear button', () => {
    const onClear = vi.fn()
    const { container } = render(<Input onClear={onClear} />)
    fireEvent.change(container.querySelector('input')!, {
      target: { value: 'test' },
    })
    fireEvent.click(container.querySelector('[aria-label="clear-button"]')!)
    expect(onClear).toHaveBeenCalled()
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
})
