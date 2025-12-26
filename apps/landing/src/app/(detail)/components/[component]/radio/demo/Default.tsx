/**
 * ## Default
 * Use `RadioGroup` with an `options` array to create a group of radio buttons.
 * Only one option can be selected at a time.
 */
'use client'

import { RadioGroup } from '@devup-ui/components'

export default function Default() {
  return (
    <RadioGroup
      defaultValue="option1"
      options={[
        { value: 'option1', label: 'Option 1' },
        { value: 'option2', label: 'Option 2' },
        { value: 'option3', label: 'Option 3' },
      ]}
    />
  )
}
