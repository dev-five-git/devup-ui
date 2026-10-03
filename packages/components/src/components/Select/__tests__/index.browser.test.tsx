import { css, Flex } from '@devup-ui/react'
import { describe, expect, it, mock } from 'bun:test'
import { fireEvent, render } from 'bun-test-env-dom'

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
      <Select id="nested" type="radio">
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
  it('should render', () => {
    const { container } = render(<Select id="select">{children}</Select>)
    expect(container).toMatchSnapshot()
  })

  it('should throw error when used outside of Select context', () => {
    expect(() => {
      render(<SelectOption />)
    }).toThrow()
  })

  it('should require one element when SelectTrigger uses asChild', () => {
    expect(() => {
      render(
        <Select id="select">
          <SelectTrigger asChild>Text</SelectTrigger>
        </Select>,
      )
    }).toThrow('SelectTrigger with asChild requires a single element')
  })

  it('should close select when clicking outside', () => {
    const { container } = render(
      <div data-testid="container">
        <Select id="select">{children}</Select>
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
    const onOpenChange = mock()
    const { container } = render(
      <Select id="select" onOpenChange={onOpenChange} type="radio">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    expect(onOpenChange).toHaveBeenCalledWith(true)
  })

  it('should call onValueChange function when it is provided', () => {
    const onValueChange = mock()
    const { container } = render(
      <Select id="select" onChange={onValueChange} type="radio">
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
    const { container } = render(
      <Select id="select" type="default">
        {children}
      </Select>,
    )
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
    const { container } = render(
      <Select id="select" type="radio">
        {children}
      </Select>,
    )
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
    const { container } = render(
      <Select id="select" type="checkbox">
        {children}
      </Select>,
    )
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
    const { container } = render(
      <Select id="select" type="checkbox">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toBeInTheDocument()
    fireEvent.click(option2!)
    fireEvent.click(option2!)
    expect(option2?.querySelector('svg')).toBeNull()
  })

  it('should call onClick function when it is provided to SelectOption', () => {
    const onClick = mock()
    const { container } = render(
      <Select id="select">
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
      <Select defaultValue="Option 2" id="select" type="radio">
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
    const { container } = render(
      <Select id="select" type="radio">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const selectContainer = container.querySelector(
      '[aria-label="Select container"]',
    )
    expect(selectContainer).toBeInTheDocument()
    expect(selectContainer?.querySelectorAll('svg')).toHaveLength(1)
  })

  it('should have 10px gap in an option when type is checkbox', () => {
    const { container } = render(
      <Select id="select" type="checkbox">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toHaveClass('gap-0-10px--1')
  })

  it('should have 6px gap in an option when type is radio', () => {
    const { container } = render(
      <Select id="select" type="radio">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toHaveClass('gap-0-6px--1')
  })

  it('should have 0 gap in an option when type is default', () => {
    const { container } = render(
      <Select id="select" type="default">
        {children}
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option2 = container.querySelector('[data-value="Option 2"]')
    expect(option2).toHaveClass('gap-0-0--1')
  })

  it('should have undefined gap when type is not right', () => {
    const invalidTypeProps = {
      type: 'no-type',
      children,
    } as unknown as React.ComponentProps<typeof Select>
    const { container } = render(<Select id="select" {...invalidTypeProps} />)
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
        id="select"
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
      <Select defaultValue={['Option 1']} id="select" type="checkbox">
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
      'color-0-var_lp_--inputDisabledText_cm_light-dark_lp__h_E5E5E5_cm__h_373737_rp__rp_--255',
    )
  })

  it('should show confirm button when type is checkbox and showConfirmButton is true', () => {
    const { container } = render(
      <Select id="select" type="checkbox">
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
      <Select id="select" type="checkbox">
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
      <Select id="select" type="checkbox">
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
      <Select defaultValue={['Option 2']} id="select" type="checkbox">
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
      <Select defaultOpen id="select" type="checkbox">
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
        id="select"
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
    const onValueChange = mock()
    const { container } = render(
      <Select
        id="select"
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
      <Select aria-label="Select" id="select">
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
    const { container, rerender } = render(
      <Select id="select">{children}</Select>,
    )

    // open selectContainer
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)

    const selectContainer = container.querySelector(
      '[aria-label="Select container"]',
    )! as HTMLDivElement

    // happy-dom default viewport 1024x768
    // offsetHeight > 768px
    Object.defineProperty(selectContainer, 'offsetHeight', { value: 800 })
    // offsetWidth > 1024px
    Object.defineProperty(selectContainer, 'offsetWidth', { value: 1100 })

    // the container repositions when the window resizes
    window.dispatchEvent(new Event('resize'))
    rerender(<Select id="select">{children}</Select>)

    expect(container).toMatchSnapshot()
  })

  it('should change value when clicking on SelectOption without value prop', () => {
    const onChange = mock()
    const { container } = render(
      <Select className="test" id="select" onChange={onChange}>
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer>
          <SelectOption>Option 1</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const selectToggle = container.querySelector('[aria-label="Select toggle"]')
    fireEvent.click(selectToggle!)
    const option1 = container.querySelector('[aria-label="Select option"]')
    fireEvent.click(option1!)
    expect(onChange).not.toHaveBeenCalled()
    expect(container.querySelector('.test')).toHaveClass('test')
  })

  it('should render with typography prop', () => {
    const onChange = mock()
    const { container } = render(
      <Select id="select" onChange={onChange} typography="body1">
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer>
          <SelectOption>Option 1</SelectOption>
        </SelectContainer>
      </Select>,
    )
    expect(container).toMatchSnapshot()
  })
  it('is a listbox operated by keyboard', () => {
    const onChange = mock()
    const { container, getByRole, getAllByRole } = render(
      <Select id="kbd" onChange={onChange} type="radio">
        <SelectTrigger>Select</SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">a</SelectOption>
          <SelectOption disabled value="b">
            b
          </SelectOption>
          <SelectOption value="c">c</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const trigger = container.querySelector<HTMLElement>(
      '[aria-haspopup=listbox]',
    )!
    expect(trigger).toHaveAttribute('aria-controls', 'kbd-listbox')
    fireEvent.keyDown(trigger, { key: 'Tab' })
    expect(trigger).toHaveAttribute('aria-expanded', 'false')
    fireEvent.keyDown(trigger, { key: 'ArrowDown' })
    expect(getByRole('listbox')).toHaveAttribute('id', 'kbd-listbox')
    fireEvent.keyDown(trigger, { key: 'ArrowDown' })
    const [a, b, c] = getAllByRole('option')
    expect(document.activeElement).toBe(a)
    expect(b).toHaveAttribute('aria-disabled', 'true')
    const listbox = getByRole('listbox')
    fireEvent.keyDown(listbox, { key: 'ArrowDown' })
    expect(document.activeElement).toBe(c)
    fireEvent.keyDown(listbox, { key: 'ArrowDown' })
    expect(document.activeElement).toBe(a)
    fireEvent.keyDown(listbox, { key: 'ArrowUp' })
    expect(document.activeElement).toBe(c)
    fireEvent.keyDown(listbox, { key: 'Home' })
    expect(document.activeElement).toBe(a)
    fireEvent.keyDown(listbox, { key: 'End' })
    expect(document.activeElement).toBe(c)
    fireEvent.keyDown(listbox, { key: 'x' })
    fireEvent.keyDown(c, { key: 'x' })
    fireEvent.keyDown(b, { key: 'Enter' })
    expect(onChange).not.toHaveBeenCalled()
    fireEvent.keyDown(c, { key: 'Enter' })
    expect(onChange).toHaveBeenCalledWith('c')
    expect(document.activeElement).toBe(trigger)
    expect(container.querySelector('[role=listbox]')).toBeNull()
    fireEvent.keyDown(trigger, { key: 'ArrowUp' })
    expect(document.activeElement).toBe(getAllByRole('option')[2])
    expect(getAllByRole('option')[2]).toHaveAttribute('aria-selected', 'true')
    fireEvent.keyDown(getByRole('listbox'), { key: 'Escape' })
    expect(container.querySelector('[role=listbox]')).toBeNull()
    expect(document.activeElement).toBe(trigger)
  })

  it('opens with ArrowDown on an asChild trigger and marks checkbox lists', () => {
    const { container, getByRole } = render(
      <Select type="checkbox">
        <SelectTrigger asChild>
          <button type="button">t</button>
        </SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">a</SelectOption>
        </SelectContainer>
      </Select>,
    )
    fireEvent.keyDown(container.querySelector('button')!, { key: 'ArrowDown' })
    expect(getByRole('listbox')).toHaveAttribute('aria-multiselectable', 'true')
    fireEvent.keyDown(getByRole('option'), { key: ' ' })
    expect(getByRole('option')).toHaveAttribute('aria-selected', 'true')
  })

  it('asks a controlled owner to close on an outside click', () => {
    const onOpenChange = mock()
    const { container } = render(
      <Select onOpenChange={onOpenChange} open>
        <SelectTrigger>t</SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">a</SelectOption>
        </SelectContainer>
      </Select>,
    )
    fireEvent.click(document.body)
    expect(onOpenChange).toHaveBeenCalledWith(false)
    expect(container.querySelector('[role=listbox]')).not.toBeNull()
  })

  it('closes an uncontrolled select on an outside click', () => {
    const { container } = render(
      <Select defaultOpen>
        <SelectTrigger>t</SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">a</SelectOption>
        </SelectContainer>
      </Select>,
    )
    fireEvent.click(container.querySelector('[role=option]')!.parentElement!)
    expect(container.querySelector('[role=listbox]')).not.toBeNull()
    fireEvent.click(document.body)
    expect(container.querySelector('[role=listbox]')).toBeNull()
  })
})
