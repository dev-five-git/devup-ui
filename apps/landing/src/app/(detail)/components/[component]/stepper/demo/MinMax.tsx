/**
 * ## Min/Max
 * Set `min` and `max` props to limit the value range. The decrease button is disabled
 * at min value, and the increase button is disabled at max value.
 */
'use client'

import {
  Stepper,
  StepperContainer,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
} from '@devup-ui/components'

export default function MinMax() {
  return (
    <Stepper defaultValue={5} max={10} min={0}>
      <StepperContainer>
        <StepperDecreaseButton />
        <StepperInput />
        <StepperIncreaseButton />
      </StepperContainer>
    </Stepper>
  )
}
