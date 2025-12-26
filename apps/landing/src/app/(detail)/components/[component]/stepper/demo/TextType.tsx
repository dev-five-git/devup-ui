/**
 * ## Text Type
 * Set `type="text"` to display the value as read-only text instead of an editable input.
 * Users can only change the value using the increase/decrease buttons.
 */
'use client'

import {
  Stepper,
  StepperContainer,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
} from '@devup-ui/components'

export default function TextType() {
  return (
    <Stepper defaultValue={5} type="text">
      <StepperContainer>
        <StepperDecreaseButton />
        <StepperInput />
        <StepperIncreaseButton />
      </StepperContainer>
    </Stepper>
  )
}
