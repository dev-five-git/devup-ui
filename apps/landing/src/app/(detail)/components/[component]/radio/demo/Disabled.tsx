/**
 * ## Disabled
 * Set `disabled` prop to prevent user interaction with all radio options.
 * All radios will have a muted appearance and cannot be selected.
 */
'use client'

import { RadioGroup } from '@devup-ui/components'

export default function Disabled() {
  return (
    <RadioGroup
      defaultValue="option1"
      disabled
      options={[
        { value: 'option1', label: 'Option 1' },
        { value: 'option2', label: 'Option 2' },
        { value: 'option3', label: 'Option 3' },
      ]}
    />
  )
}
