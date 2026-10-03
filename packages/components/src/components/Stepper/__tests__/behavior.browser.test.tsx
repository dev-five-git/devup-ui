import { describe, expect, it, mock } from 'bun:test'
import { fireEvent, render } from 'bun-test-env-dom'

import {
  Stepper,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
} from '../index'

function Parts(props: React.ComponentProps<typeof Stepper>) {
  return (
    <Stepper {...props}>
      <StepperDecreaseButton />
      <StepperInput />
      <StepperIncreaseButton />
    </Stepper>
  )
}

const value = (container: HTMLElement) =>
  container.querySelector<HTMLInputElement>('[aria-label="Stepper value"]')!
    .value
const increase = (container: HTMLElement) =>
  container.querySelector<HTMLButtonElement>('[aria-label="Increase button"]')!
const decrease = (container: HTMLElement) =>
  container.querySelector<HTMLButtonElement>('[aria-label="Decrease button"]')!

describe('Stepper values', () => {
  it('counts when only a callback is given, and tells it every value', () => {
    const onValueChange = mock()
    const { container } = render(<Parts onValueChange={onValueChange} />)

    fireEvent.click(increase(container))
    fireEvent.click(increase(container))

    expect(value(container)).toBe('2')
    expect(onValueChange.mock.calls).toEqual([[1], [2]])
  })

  it('shows what the owner of a controlled stepper says, and only tells it', () => {
    const onValueChange = mock()
    const { container, rerender } = render(
      <Parts onValueChange={onValueChange} value={5} />,
    )

    fireEvent.click(increase(container))

    expect(onValueChange).toHaveBeenCalledWith(6)
    expect(value(container)).toBe('5')
    rerender(<Parts onValueChange={onValueChange} value={6} />)
    expect(value(container)).toBe('6')
  })

  it('keeps to its bounds', () => {
    const onValueChange = mock()
    const { container } = render(
      <Parts defaultValue={9} max={10} min={8} onValueChange={onValueChange} />,
    )

    fireEvent.click(increase(container))
    expect(increase(container)).toBeDisabled()
    fireEvent.click(increase(container))
    fireEvent.click(decrease(container))
    fireEvent.click(decrease(container))
    fireEvent.click(decrease(container))

    expect(value(container)).toBe('8')
    expect(onValueChange.mock.calls).toEqual([[10], [9], [8]])
    expect(decrease(container)).toBeDisabled()
  })

  it('takes a typed number into the bounds', () => {
    const { container } = render(<Parts max={10} />)
    const input = container.querySelector('input')!

    fireEvent.change(input, { target: { value: '42' } })

    expect(value(container)).toBe('10')
  })
})

describe('Stepper parts cannot lose their safeguards', () => {
  it('keeps a button disabled at its bound however the caller passes disabled', () => {
    const onClick = mock()
    const { container } = render(
      <Stepper defaultValue={0} min={0}>
        <StepperDecreaseButton disabled={false} onClick={onClick} />
        <StepperInput />
        <StepperIncreaseButton />
      </Stepper>,
    )

    expect(decrease(container)).toBeDisabled()
    fireEvent.click(decrease(container))
    expect(onClick).not.toHaveBeenCalled()
    expect(value(container)).toBe('0')
  })

  it('lets the caller disable a button that is within its bounds', () => {
    const { container } = render(
      <Stepper defaultValue={3}>
        <StepperDecreaseButton disabled />
        <StepperInput />
        <StepperIncreaseButton disabled />
      </Stepper>,
    )

    expect(decrease(container)).toBeDisabled()
    expect(increase(container)).toBeDisabled()
  })

  it('runs the click handler of the caller and still steps', () => {
    const onClick = mock()
    const { container } = render(
      <Stepper defaultValue={3}>
        <StepperDecreaseButton onClick={onClick} />
        <StepperInput />
        <StepperIncreaseButton onClick={onClick} />
      </Stepper>,
    )

    fireEvent.click(increase(container))
    fireEvent.click(decrease(container))
    fireEvent.click(decrease(container))

    expect(onClick).toHaveBeenCalledTimes(3)
    expect(value(container)).toBe('2')
  })

  it('leaves the step to the caller who prevents the click', () => {
    const { container } = render(
      <Stepper defaultValue={3}>
        <StepperDecreaseButton onClick={(e) => e.preventDefault()} />
        <StepperInput />
        <StepperIncreaseButton onClick={(e) => e.preventDefault()} />
      </Stepper>,
    )

    fireEvent.click(increase(container))
    fireEvent.click(decrease(container))

    expect(value(container)).toBe('3')
  })

  it('runs the change handler of the caller and still takes the number', () => {
    const onChange = mock()
    const { container } = render(
      <Stepper>
        <StepperInput onChange={onChange} />
      </Stepper>,
    )

    fireEvent.change(container.querySelector('input')!, {
      target: { value: '7' },
    })

    expect(onChange).toHaveBeenCalledTimes(1)
    expect(value(container)).toBe('7')
  })

  it('does not let the caller replace the value, the type or the clear button', () => {
    const { container } = render(
      <Stepper defaultValue={4}>
        <StepperInput allowClear type="text" value="99" />
      </Stepper>,
    )
    const input = container.querySelector('input')!

    expect(input.value).toBe('4')
    expect(input.type).toBe('number')
    expect(container.querySelector('button')).toBeNull()
  })
})

describe('Stepper text mode', () => {
  it('shows the value as read-only output, without input attributes', () => {
    const { container } = render(
      <Stepper defaultValue={3} type="text">
        <StepperInput
          aria-describedby="hint"
          className="mine"
          data-extra="1"
          id="count"
          placeholder="ignored"
          readOnly={false}
          title="Count"
        />
      </Stepper>,
    )
    const output = container.querySelector('output')!

    expect(output.textContent).toBe('3')
    expect(output).toHaveClass('mine')
    expect(output).toHaveAttribute('aria-label', 'Stepper value')
    expect(output).toHaveAttribute('aria-describedby', 'hint')
    expect(output).toHaveAttribute('data-extra', '1')
    expect(output).toHaveAttribute('id', 'count')
    expect(output).toHaveAttribute('title', 'Count')
    for (const name of ['placeholder', 'readonly', 'type', 'value'])
      expect(output).not.toHaveAttribute(name)
    expect(container.querySelector('input')).toBeNull()
  })

  it('follows the buttons', () => {
    const { container } = render(
      <Stepper defaultValue={3} type="text">
        <StepperDecreaseButton />
        <StepperInput />
        <StepperIncreaseButton />
      </Stepper>,
    )

    fireEvent.click(increase(container))

    expect(container.querySelector('output')?.textContent).toBe('4')
  })
})
