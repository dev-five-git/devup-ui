import { css, Flex } from '@devup-ui/react'
import { fireEvent, render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import {
  Select,
  SelectContainer,
  SelectDivider,
  SelectOption,
  SelectTrigger,
} from '..'
import { IconArrow } from '../IconArrow'

const children = (
  <>
    <SelectTrigger>Select</SelectTrigger>
    <SelectContainer>
      <SelectOption disabled value="Option 1">
        Option 1
      </SelectOption>
      <SelectOption value="Option 2">Option 2</SelectOption>
      <SelectDivider />
      <SelectOption value="Option 3">Option 3</SelectOption>
      <SelectOption disabled value="Option 4">
        Option 4
      </SelectOption>
      <Select type="radio">
        <SelectTrigger asChild>
          <SelectOption>
            <Flex alignItems="center" justifyContent="space-between" w="100%">
              Option 5<IconArrow />
            </Flex>
          </SelectOption>
        </SelectTrigger>
        <SelectContainer
          className={css({
            right: '0',
            top: '0',
            transform: 'translateX(100%)',
          })}
        >
          <SelectOption value="Option 6">Option 6</SelectOption>
          <SelectOption value="Option 7">Option 7</SelectOption>
        </SelectContainer>
      </Select>
    </SelectContainer>
  </>
)

describe('Select', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  it('should render', () => {
    const { container } = render(<Select>{children}</Select>)
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
        <Select>{children}</Select>
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
      <Select onOpenChange={onOpenChange} type="radio">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    expect(onOpenChange).toHaveBeenCalledWith(true)
  })

  it('should call onValueChange function when it is provided', () => {
    const onValueChange = vi.fn()
    const { container } = render(
      <Select onChange={onValueChange} type="radio">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    fireEvent.click(option2!)
    expect(onValueChange).toHaveBeenCalledWith('Option 2')
  })

  it('should do nothing when onValueChange is not provided and type is default', () => {
    const { container } = render(<Select type="default">{children}</Select>)
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
    const { container } = render(<Select type="radio">{children}</Select>)
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
    const { container } = render(<Select type="checkbox">{children}</Select>)
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
    const { container } = render(<Select type="checkbox">{children}</Select>)
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
      <Select defaultValue="Option 2" type="radio">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    expect(option2?.querySelector('svg')).toBeInTheDocument()
  })

  it('should not have a check mark when type is radio and defaultValue is not provided', () => {
    const { container } = render(<Select type="radio">{children}</Select>)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const selectContainer = container.querySelector(
      '[aria-label="Select container"]',
    )
    expect(selectContainer).toBeInTheDocument()
    expect(selectContainer?.querySelectorAll('svg')).toHaveLength(1)
  })

  it('should have 10px gap in an option when type is checkbox', () => {
    const { container } = render(<Select type="checkbox">{children}</Select>)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toHaveClass('gap-0-10px--1')
  })

  it('should have 6px gap in an option when type is radio', () => {
    const { container } = render(<Select type="radio">{children}</Select>)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toHaveClass('gap-0-6px--1')
  })

  it('should have 0 gap in an option when type is default', () => {
    const { container } = render(<Select type="default">{children}</Select>)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toHaveClass('gap-0-0--1')
  })

  it('should have undefined gap when type is not right', () => {
    // @ts-expect-error - test for wrong type
    const { container } = render(<Select type="no-type">{children}</Select>)
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).not.toHaveClass('gap-0-0--1')
  })

  it('should add styleVars to the container when colors are provided', () => {
    const { container } = render(
      <Select
        colors={{
          primary: 'red',
          border: 'blue',
          inputBackground: 'green',
          base10: 'yellow',
          title: 'purple',
        }}
        data-testid="select"
      >
        {children}
      </Select>,
    )
    const select = container.querySelector('[data-testid="select"]')
    expect(select).toHaveStyle({
      '--primary': 'red',
      '--border': 'blue',
      '--inputBackground': 'green',
      '--base10': 'yellow',
      '--title': 'purple',
    })
  })

  it('should have disabled check color when type is checkbox and the option is disabled', () => {
    const { container } = render(
      <Select defaultValue={['Option 1']} type="checkbox">
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer>
          <SelectOption disabled value="Option 1">
            Option 1
          </SelectOption>
        </SelectContainer>
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option1 = container.querySelector('[data-value="Option 1"]')
    expect(option1?.querySelector('svg')).toHaveClass(
      'color-0-var(--inputDisabledText,light-dark(#E5E5E5,#373737))--255',
    )
  })

  it('should show confirm button when type is checkbox and showConfirmButton is true', () => {
    const { container } = render(
      <Select type="checkbox">
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer showConfirmButton>
          <SelectOption disabled value="Option 1">
            Option 1
          </SelectOption>
          <SelectDivider />
          <SelectOption value="Option 2">Option 2</SelectOption>
          <SelectOption value="Option 3">Option 3</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const confirmButton = container.querySelector(
      '[aria-label="Select confirm button"]',
    )
    expect(confirmButton).toBeInTheDocument()
  })

  it('should close select when clicking confirm button', () => {
    const { container } = render(
      <Select type="checkbox">
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer showConfirmButton>
          <SelectOption value="Option 1">Option 1</SelectOption>
          <SelectOption value="Option 2">Option 2</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const confirmButton = container.querySelector(
      '[aria-label="Select confirm button"]',
    )
    fireEvent.click(confirmButton!)
    expect(selectToggle).toHaveAttribute('aria-expanded', 'false')
  })

  it('should not show confirm button when type is checkbox and showConfirmButton is false', () => {
    const { container } = render(
      <Select type="checkbox">
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer showConfirmButton={false}>
          <SelectOption disabled value="Option 1">
            Option 1
          </SelectOption>
          <SelectDivider />
          <SelectOption value="Option 2">Option 2</SelectOption>
          <SelectOption value="Option 3">Option 3</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const confirmButton = container.querySelector(
      '[aria-label="Select confirm button"]',
    )
    expect(confirmButton).not.toBeInTheDocument()
  })

  it('should render IconCheck when type is checkbox and the option is selected', () => {
    const { container } = render(
      <Select defaultValue={['Option 2']} type="checkbox">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2?.querySelector('svg')).toBeInTheDocument()
  })

  it('should not check the option when type is checkbox and the option is not selected', () => {
    const { container } = render(
      <Select defaultOpen type="checkbox">
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer showConfirmButton={false}>
          <SelectOption disabled value="Option 1">
            Option 1
          </SelectOption>
          <SelectDivider />
          <SelectOption value="Option 2">Option 2</SelectOption>
          <SelectOption value="Option 3">Option 3</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const svg = container.querySelector('svg')
    expect(svg).not.toBeInTheDocument()
  })

  it('should render with options properties', () => {
    const { container } = render(
      <Select
        options={[
          { label: 'Option 1', value: 'Option 1' },
          { value: 'Option 2' },
        ]}
      >
        Select
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option1 = container.querySelector('[data-value="Option 1"]')
    expect(option1).toBeInTheDocument()
  })

  it('should call onChange function when it is provided to SelectOption', () => {
    const onValueChange = vi.fn()
    const { container } = render(
      <Select
        onChange={onValueChange}
        options={[
          { label: 'Option 1', value: 'Option 1' },
          { value: 'Option 2' },
        ]}
      >
        Select
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    fireEvent.click(option2!)
    expect(onValueChange).toHaveBeenCalledWith('Option 2')
  })

  it('should render with x and y properties', () => {
    const { container } = render(
      <Select>
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer x={10} y={10}>
          <SelectOption value="Option 1">Option 1</SelectOption>
          <SelectOption value="Option 2">Option 2</SelectOption>
        </SelectContainer>
      </Select>,
    )
    expect(container).toMatchSnapshot()
  })

  it('should render with overflow screen', () => {
    const { container } = render(
      <Select
        className={css({
          pos: 'fixed',
          bottom: '0px',
          right: '0px',
        })}
      >
        {children}
      </Select>,
    )
    expect(container).toMatchSnapshot()
  })
  it('should render with overflow screen', () => {
    const { container } = render(
      <Select
        className={css({
          pos: 'fixed',
          bottom: '0px',
        })}
      >
        {children}
      </Select>,
    )
    expect(container).toMatchSnapshot()
  })
  it('should render with overflow screen', () => {
    const { container } = render(
      <Select
        className={css({
          pos: 'fixed',
          right: '0px',
        })}
      >
        {children}
      </Select>,
    )
    expect(container).toMatchSnapshot()
  })
})
