/**
 * ## Default
 * Compound component with editable input. Use `StepperContainer` to wrap the buttons and input,
 * and compose with `StepperDecreaseButton`, `StepperInput`, and `StepperIncreaseButton`.
 */
'use client'

import {
  Stepper,
  StepperContainer,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
} from '@devup-ui/components'

export default function Default() {
  return (
    <Stepper>
      <StepperContainer>
        <StepperDecreaseButton />
        <StepperInput />
        <StepperIncreaseButton />
      </StepperContainer>
    </Stepper>
  )
}
