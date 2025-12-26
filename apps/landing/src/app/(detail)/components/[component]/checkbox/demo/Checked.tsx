/**
 * ## Checked
 * Use `defaultChecked` prop to set the initial checked state for uncontrolled usage,
 * or use `checked` prop for controlled state management.
 */
'use client'

import { Checkbox } from '@devup-ui/components'

export default function Checked() {
  return <Checkbox defaultChecked>Checked by default</Checkbox>
}
