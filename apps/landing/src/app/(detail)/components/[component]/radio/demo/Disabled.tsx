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
