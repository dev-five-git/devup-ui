import { fireEvent, render } from '@testing-library/react'

import {
  Stepper,
  StepperContainer,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
} from '..'

describe('Stepper', () => {
  it('should render', () => {
    const { container } = render(
      <Stepper>
        <StepperContainer>
          <StepperDecreaseButton />
          <StepperInput />
          <StepperIncreaseButton />
        </StepperContainer>
      </Stepper>,
    )
    expect(container).toMatchSnapshot()
  })

  it('should throw error if children are used outside of StepperProvider', () => {
    expect(() => {
      render(<StepperInput />)
    }).toThrow('useStepper must be used within a StepperProvider')
  })

  it('should call onValueChange when value is changed', () => {
    const onValueChange = vi.fn()
    const { container } = render(
      <Stepper onValueChange={onValueChange}>
        <StepperInput />
      </Stepper>,
    )
    const input = container.querySelector('[aria-label="Stepper value"]')
    fireEvent.change(input!, { target: { value: '10' } })
    expect(onValueChange).toHaveBeenCalledWith(10)
  })

  it('should change inner value when onValueChange is not provided', () => {
    const { container } = render(
      <Stepper>
        <StepperInput />
      </Stepper>,
    )
    const input = container.querySelector('[aria-label="Stepper value"]')
    fireEvent.change(input!, { target: { value: '10' } })
    expect(input).toHaveAttribute('data-value', '10')
  })

  it('should not change inner value by clicking the number when type is text', () => {
    const { container } = render(
      <Stepper type="text">
        <StepperInput />
      </Stepper>,
    )
    const input = container.querySelector('[aria-label="Stepper value"]')
    expect(input?.nodeName).toBe('DIV')
    expect(input).toHaveAttribute('data-value', '0')
  })

  it('should have disabled decrease button when value is at min', () => {
    const { container } = render(
      <Stepper>
        <StepperDecreaseButton />
        <StepperInput />
        <StepperIncreaseButton />
      </Stepper>,
    )
    const decreaseButton = container.querySelector(
      '[aria-label="Decrease button"] svg',
    )
    fireEvent.change(container.querySelector('[aria-label="Stepper value"]')!, {
      target: { value: '0' },
    })
    expect(decreaseButton).toHaveClass(
      'color-0-var(--base10,light-dark(#0000001A,#FFFFFF1A))--255',
    )
  })

  it('should have disabled increase button when value is at max', () => {
    const { container } = render(
      <Stepper>
        <StepperDecreaseButton />
        <StepperInput />
        <StepperIncreaseButton />
      </Stepper>,
    )
    const increaseButton = container.querySelector(
      '[aria-label="Increase button"] svg',
    )
    fireEvent.change(container.querySelector('[aria-label="Stepper value"]')!, {
      target: { value: '100' },
    })
    expect(increaseButton).toHaveClass(
      'color-0-var(--base10,light-dark(#0000001A,#FFFFFF1A))--255',
    )
  })

  it('should export components', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      Stepper: expect.any(Function),
      StepperContainer: expect.any(Function),
      StepperDecreaseButton: expect.any(Function),
      StepperIncreaseButton: expect.any(Function),
      StepperInput: expect.any(Function),
      useStepper: expect.any(Function),
    })
  })

  it('should increase value when increase button is clicked', () => {
    const { container } = render(
      <Stepper>
        <StepperDecreaseButton />
        <StepperInput />
        <StepperIncreaseButton />
      </Stepper>,
    )
    const increaseButton = container.querySelector(
      '[aria-label="Increase button"]',
    )
    fireEvent.click(increaseButton!)
    expect(
      container.querySelector('[aria-label="Stepper value"]'),
    ).toHaveAttribute('data-value', '1')
  })

  it('should decrease value when decrease button is clicked', () => {
    const { container } = render(
      <Stepper defaultValue={1}>
        <StepperDecreaseButton />
        <StepperInput />
        <StepperIncreaseButton />
      </Stepper>,
    )
    const decreaseButton = container.querySelector(
      '[aria-label="Decrease button"]',
    )
    fireEvent.click(decreaseButton!)
    expect(
      container.querySelector('[aria-label="Stepper value"]'),
    ).toHaveAttribute('data-value', '0')
  })
})
