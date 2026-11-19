import { ProgressBar } from '@devup-ui/components'

/**
 * **Step**
 * Use `variant="step"` for a multi-step flow. `max` is the number of steps and `value` the steps done; each step gets its own segment.
 */
export default function Step() {
  return (
    <ProgressBar
      aria-label="Sign-up steps"
      max={6}
      showValue
      value={3}
      variant="step"
    />
  )
}
