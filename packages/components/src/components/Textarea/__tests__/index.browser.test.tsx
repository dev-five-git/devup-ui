import { DevupThemeTypography } from '@devup-ui/react'
import { describe, expect, it } from 'bun:test'
import { render, userEvent } from 'bun-test-env-dom'

import { Textarea } from '..'

describe('Textarea', () => {
  it('should render with default props', () => {
    const { container } = render(<Textarea />)
    expect(container).toMatchSnapshot()
  })

  it('should render with placeholder', () => {
    const { container } = render(<Textarea placeholder="Enter text..." />)
    expect(container.querySelector('textarea')).toHaveAttribute(
      'placeholder',
      'Enter text...',
    )
  })

  it('should render with default value', () => {
    const { container } = render(<Textarea defaultValue="Default text" />)
    expect(container.querySelector('textarea')?.value).toBe('Default text')
  })

  it('should render with disabled prop', () => {
    const { container } = render(<Textarea disabled />)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('textarea')).toHaveAttribute('disabled')
  })

  it('should render error style when error is true', () => {
    const { container } = render(<Textarea error />)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('textarea')).toHaveClass(
      'border-color-0-var_lp_--error_cm_light-dark_lp__h_D52B2E_cm__h_FF5B5E_rp__rp_-_a__lb_aria-invalid_eq__dq_true_dq__rb_-1',
    )
  })

  it('should render with error message', () => {
    const { container } = render(
      <Textarea error errorMessage="Error message" id="field" />,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('[role="alert"]')).toBeInTheDocument()
    expect(container.querySelector('[role="alert"]')?.textContent).toBe(
      'Error message',
    )
  })

  it('should not render error message when error is false', () => {
    const { container } = render(<Textarea errorMessage="Error message" />)
    expect(container.querySelector('[role="alert"]')).not.toBeInTheDocument()
  })

  it('should render with aria-invalid when error is true', () => {
    const { container } = render(<Textarea error />)
    expect(container.querySelector('textarea')).toHaveAttribute(
      'aria-invalid',
      'true',
    )
  })

  it('should not have aria-invalid when error is false', () => {
    const { container } = render(<Textarea />)
    expect(
      container.querySelector('textarea')?.hasAttribute('aria-invalid'),
    ).toBe(false)
  })

  it('should render with custom rows', () => {
    const { container } = render(<Textarea rows={6} />)
    expect(container.querySelector('textarea')).toHaveAttribute('rows', '6')
  })

  it('should pass colors prop', () => {
    const { container } = render(
      <Textarea
        colors={{
          primary: 'red',
          error: 'blue',
          text: 'green',
        }}
        error
        errorMessage="wrong"
      />,
    )
    // On the container, so the message sees them too
    const root = container.firstElementChild
    expect(root).toHaveStyle({
      '--primary': 'red',
      '--error': 'blue',
      '--text': 'green',
    })
    for (const part of [
      container.querySelector('textarea'),
      container.querySelector('[role="alert"]'),
    ])
      expect(part?.closest('[style*="--error"]')).toBe(root)
  })

  it('should have typography when typography is provided', () => {
    const { container } = render(
      <Textarea typography={'inlineLabelS' as keyof DevupThemeTypography} />,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('textarea')).toHaveClass('typo-inlineLabelS')
  })

  it('should pass className prop', () => {
    const { container } = render(<Textarea className="custom-class" />)
    expect(container.querySelector('textarea')).toHaveClass('custom-class')
  })

  it('should pass classNames.container prop', () => {
    const { container } = render(
      <Textarea classNames={{ container: 'container-class' }} />,
    )
    expect(container.firstChild).toHaveClass('container-class')
  })

  it('should pass classNames.textarea prop', () => {
    const { container } = render(
      <Textarea classNames={{ textarea: 'textarea-class' }} />,
    )
    expect(container.querySelector('textarea')).toHaveClass('textarea-class')
  })

  it('should pass classNames.errorMessage prop', () => {
    const { container } = render(
      <Textarea
        classNames={{ errorMessage: 'error-class' }}
        error
        errorMessage="Error"
        id="field"
      />,
    )
    expect(container.querySelector('[role="alert"]')).toHaveClass('error-class')
  })

  it('should handle onChange event', async () => {
    let value = ''
    const { container } = render(
      <Textarea onChange={(e) => (value = e.target.value)} />,
    )
    await userEvent.type(container.querySelector('textarea')!, 'new value')
    expect(value).toBe('new value')
  })

  it('should render full width by default', () => {
    const { container } = render(<Textarea />)
    expect(container.querySelector('textarea')).toHaveClass(
      'width-0-100_pc_--1',
    )
  })
})
