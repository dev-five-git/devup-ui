import { describe, expect, it } from 'bun:test'
import { fireEvent, render } from 'bun-test-env-dom'
import { createRef } from 'react'

import {
  Button,
  Checkbox,
  Input,
  Radio,
  RadioGroup,
  Select,
  SelectContainer,
  SelectDivider,
  SelectOption,
  SelectTrigger,
  Stepper,
  StepperContainer,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
  Textarea,
  Toggle,
} from '../index'

describe('forwardRef', () => {
  it('links both labels to the forwarded checkbox when a custom id is given', () => {
    // Given
    const ref = createRef<HTMLInputElement>()

    // When
    const { container } = render(
      <Checkbox ref={ref} id="custom-checkbox">
        Checkbox
      </Checkbox>,
    )

    // Then
    expect(ref.current?.id).toBe('custom-checkbox')
    const labels = container.querySelectorAll('label')
    expect(labels).toHaveLength(2)
    for (const label of labels) {
      expect(label.htmlFor).toBe(ref.current?.id)
    }
  })

  it('hands its ref to the native radio when the button variant is used', () => {
    // Given
    const ref = createRef<HTMLInputElement>()

    // When
    render(
      <Radio ref={ref} variant="button">
        Radio
      </Radio>,
    )

    // Then
    expect(ref.current?.type).toBe('radio')
  })

  it('hands each component ref to its form element', () => {
    const button = createRef<HTMLButtonElement>()
    const input = createRef<HTMLInputElement>()
    const radio = createRef<HTMLInputElement>()
    const textarea = createRef<HTMLTextAreaElement>()
    render(
      <>
        <Button ref={button}>b</Button>
        <Input ref={input} />
        <Radio ref={radio}>r</Radio>
        <Textarea ref={textarea} />
      </>,
    )
    expect(button.current?.tagName).toBe('BUTTON')
    expect(input.current?.tagName).toBe('INPUT')
    expect(radio.current?.type).toBe('radio')
    expect(textarea.current?.tagName).toBe('TEXTAREA')
  })

  it('hands the switch element to the ref of a toggle', () => {
    const ref = createRef<HTMLDivElement>()
    const { container } = render(<Toggle ref={ref} />)
    expect(ref.current).toBe(container.querySelector('[role=switch]'))
  })

  it('hands the group element to the ref of a radio group', () => {
    const ref = createRef<HTMLDivElement>()
    const { container } = render(
      <RadioGroup ref={ref} options={[{ value: 1, label: 'One' }]} />,
    )
    expect(ref.current).toBe(container.querySelector('[role=radiogroup]'))
  })

  it('hands each select part its element and keeps the select working', () => {
    const root = createRef<HTMLDivElement>()
    const listbox = createRef<HTMLDivElement>()
    const option = createRef<HTMLDivElement>()
    const divider = createRef<HTMLDivElement>()
    const { container } = render(
      <Select ref={root} defaultOpen>
        <SelectTrigger>Open</SelectTrigger>
        <SelectContainer ref={listbox}>
          <SelectOption ref={option} value="a">
            A
          </SelectOption>
          <SelectDivider ref={divider} />
        </SelectContainer>
      </Select>,
    )
    expect(container.firstElementChild).toBe(root.current)
    expect(listbox.current).toBe(container.querySelector('[role=listbox]'))
    expect(option.current).toBe(container.querySelector('[role=option]'))
    expect(divider.current?.parentElement).toBe(listbox.current)

    // The select still sees its own root: a click inside keeps it open,
    // a click outside closes it
    fireEvent.click(root.current!)
    expect(container.querySelector('[role=listbox]')).toBeInTheDocument()
    fireEvent.click(document.body)
    expect(container.querySelector('[role=listbox]')).toBeNull()
    expect(listbox.current).toBeNull()
  })

  it('hands each stepper part its element', () => {
    const box = createRef<HTMLDivElement>()
    const decrease = createRef<HTMLButtonElement>()
    const increase = createRef<HTMLButtonElement>()
    const input = createRef<HTMLElement>()
    const output = createRef<HTMLElement>()
    render(
      <>
        <Stepper>
          <StepperContainer ref={box}>
            <StepperDecreaseButton ref={decrease} />
            <StepperInput ref={input} />
            <StepperIncreaseButton ref={increase} />
          </StepperContainer>
        </Stepper>
        <Stepper type="text">
          <StepperInput ref={output} />
        </Stepper>
      </>,
    )
    expect(box.current?.contains(input.current)).toBe(true)
    expect(decrease.current?.getAttribute('aria-label')).toBe('Decrease button')
    expect(increase.current?.getAttribute('aria-label')).toBe('Increase button')
    expect(input.current?.tagName).toBe('INPUT')
    expect(output.current?.tagName).toBe('OUTPUT')
  })
})
