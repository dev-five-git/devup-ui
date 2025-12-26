/**
 * ## Column Direction
 * Set `direction="column"` to stack radio buttons vertically.
 * Default direction is `"row"` which displays options horizontally.
 */
'use client'

import { RadioGroup } from '@devup-ui/components'

export default function Column() {
  return (
    <RadioGroup
      defaultValue="option1"
      direction="column"
      options={[
        { value: 'option1', label: 'Option 1' },
        { value: 'option2', label: 'Option 2' },
        { value: 'option3', label: 'Option 3' },
      ]}
    />
  )
}
