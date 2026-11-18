import { describe, expect, it } from 'bun:test'

describe('export', () => {
  it('should export components', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      Button: expect.any(Object),
      Input: expect.any(Object),
      ProgressBar: expect.any(Object),
      Textarea: expect.any(Object),
      Stepper: expect.any(Function),
      Select: expect.any(Object),
      Radio: expect.any(Object),
      RadioGroup: expect.any(Object),
      SelectContainer: expect.any(Object),
      SelectDivider: expect.any(Object),
      SelectOption: expect.any(Object),
      SelectTrigger: expect.any(Object),
      StepperContainer: expect.any(Object),
      StepperDecreaseButton: expect.any(Object),
      StepperIncreaseButton: expect.any(Object),
      StepperInput: expect.any(Object),
      useStepper: expect.any(Function),
      SelectContext: expect.any(Object),
      useSelect: expect.any(Function),
      Toggle: expect.any(Object),
      Checkbox: expect.any(Object),
    })
  })
})
