'use client'

import {
  Select,
  SelectContainer,
  SelectOption,
  SelectTrigger,
} from '@devup-ui/components'
import { useState } from 'react'

/**
 * **Radio Type**
 * Use `type="radio"` for single selection mode. The selected option is highlighted with a check icon, and the dropdown closes after selection.
 */
export default function Radio() {
  const [value, setValue] = useState('')

  return (
    <Select onChange={setValue} type="radio" value={value}>
      <SelectTrigger>{value || 'Select an option'}</SelectTrigger>
      <SelectContainer>
        <SelectOption value="option1">Option 1</SelectOption>
        <SelectOption value="option2">Option 2</SelectOption>
        <SelectOption value="option3">Option 3</SelectOption>
      </SelectContainer>
    </Select>
  )
}
