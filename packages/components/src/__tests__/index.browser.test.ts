import { describe, expect, it } from 'bun:test'

describe('export', () => {
  it('should export components', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      Button: expect.any(Function),
      Input: expect.any(Function),
      Stepper: expect.any(Function),
      Select: expect.any(Function),
      Radio: expect.any(Function),
      RadioGroup: expect.any(Function),
      SelectContainer: expect.any(Function),
      SelectDivider: expect.any(Function),
      SelectOption: expect.any(Function),
      SelectTrigger: expect.any(Function),
      StepperContainer: expect.any(Function),
      StepperDecreaseButton: expect.any(Function),
      StepperIncreaseButton: expect.any(Function),
      StepperInput: expect.any(Function),
      useStepper: expect.any(Function),
      SelectContext: expect.any(Object),
      useSelect: expect.any(Function),
      Toggle: expect.any(Function),
      Checkbox: expect.any(Function),
    })
  })
})
