import { css, DevupThemeTypography } from '@devup-ui/react'
import { render } from '@testing-library/react'

import { Button } from '../index'

describe('Button', () => {
  it('should render', () => {
    const { container } = render(<Button>Click me</Button>)
    expect(container).toMatchSnapshot()
  })

  it('should render default style when variant is default', () => {
    const { container } = render(<Button variant="default">Click me</Button>)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveStyle({
      color: 'var(--text, #272727)',
    })
  })

  it('should disable', () => {
    const { container } = render(<Button disabled>Click me</Button>)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveAttribute('disabled')
  })

  it('should render error style when isError is true and variant is default', () => {
    const { container } = render(<Button isError>Click me</Button>)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveStyle({
      color: 'var(--error, #D52B2E)',
    })
  })

  it('should render primary background color when isError is true and variant is primary', () => {
    const { container } = render(
      <Button isError variant="primary">
        Click me
      </Button>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveStyle({
      backgroundColor: 'var(--primary, #8163E1)',
    })
  })

  it('should not render error color when isError is false and variant is default', async () => {
    const { getByTestId } = render(
      <Button data-testid="button">Click me</Button>,
    )
    const button = getByTestId('button')
    expect(button).toHaveStyle({
      color: 'var(--text, #272727)',
    })
    expect(button).toMatchSnapshot()
  })

  it('should have class name when className is provided', () => {
    const { container } = render(<Button className="test">Click me</Button>)
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveClass('test')
  })

  it('should not have px when a wrong size variable is provided', () => {
    const { container } = render(
      // @ts-expect-error
      <Button size="big">Click me</Button>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).not.toHaveClass('px-0-16px--1')
  })

  it('should not have bg when a wrong size variable is provided', () => {
    const { container } = render(
      // @ts-expect-error
      <Button variant="red">Click me</Button>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).not.toHaveClass(
      'bg-0-color-mix(in srgb,var(--primary,#8163E1) 10%,#FFF 90%)-8380715471663921674-1',
    )
  })

  it('should have text overflow ellipsis when ellipsis is true', () => {
    const { container } = render(
      <Button ellipsis style={{ maxWidth: '10px' }}>
        Click meClick meClick meClick meClick meClick meClick me Click meClick
        meClick meClick meClick meClick meClick me Click meClick meClick meClick
        meClick meClick meClick me Click meClick meClick meClick meClick meClick
        meClick me A1
      </Button>,
    )
    expect(container).toMatchSnapshot()
    const button = container.querySelector('button>div>div')
    expect(button).toHaveClass('textOverflow-0-ellipsis--255')
  })

  it('should have font size 15px when size is md and variant is primary', () => {
    const { container } = render(
      <Button data-testid="button" size="md" variant="primary">
        Click me
      </Button>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveClass('fontSize-0-15px--1')
  })

  it('should have font size 15px when size is sm and variant is primary', () => {
    const { container } = render(
      <Button data-testid="button" size="sm" variant="primary">
        Click me
      </Button>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveClass('fontSize-0-15px--1')
  })

  it('should have font size 14px when size is sm and variant is default', () => {
    const { container } = render(
      <Button data-testid="button" size="sm" variant="default">
        Click me
      </Button>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveClass('fontSize-0-14px--1')
  })

  it('should render icon when icon is provided', () => {
    const { container } = render(
      <Button
        data-testid="button"
        icon={
          <svg
            className={css({ color: '$white' })}
            fill="none"
            height="24"
            viewBox="0 0 25 24"
            width="25"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M13.3333 7.83333C13.3333 7.3731 12.9602 7 12.5 7C12.0398 7 11.6667 7.3731 11.6667 7.83333V11.1667H8.33333C7.8731 11.1667 7.5 11.5398 7.5 12C7.5 12.4602 7.8731 12.8333 8.33333 12.8333H11.6667V16.1667C11.6667 16.6269 12.0398 17 12.5 17C12.9602 17 13.3333 16.6269 13.3333 16.1667V12.8333H16.6667C17.1269 12.8333 17.5 12.4602 17.5 12C17.5 11.5398 17.1269 11.1667 16.6667 11.1667H13.3333V7.83333Z"
              fill="white"
            />
          </svg>
        }
      >
        Click me
      </Button>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('svg')).toBeInTheDocument()
  })
  it('color should be white', () => {
    const { container } = render(
      <Button
        className="aaaaa"
        colors={{
          primary: 'red',
          error: 'red',
          text: 'red',
          border: 'red',
          inputBackground: 'red',
          primaryFocus: 'red',
        }}
      >
        Click me
      </Button>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveStyle({
      color: 'var(--text, #272727)',
    })
  })

  it('should have typography when typography is provided', () => {
    const { container } = render(
      <Button typography={'inlineLabelS' as keyof DevupThemeTypography}>
        Click me
      </Button>,
    )
    expect(container).toMatchSnapshot()
    expect(container.querySelector('button')).toHaveClass('typo-inlineLabelS')
  })
})
