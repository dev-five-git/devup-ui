'use client'

import { useState } from 'react'
import {
  Select,
  SelectContainer,
  SelectOption,
  SelectTrigger,
} from '@devup-ui/components'

/**
 * **Checkbox Type**
 * Use `type="checkbox"` for multiple selection mode. Users can select multiple options, and use `showConfirmButton` on the container to add a confirm button.
 */
export default function Checkbox() {
  const [value, setValue] = useState<string[]>([])

  const handleChange = (nextValue: string) => {
    if (value.includes(nextValue)) {
      setValue(value.filter((v) => v !== nextValue))
    } else {
      setValue([...value, nextValue])
    }
  }

  return (
    <Select onChange={handleChange} type="checkbox" value={value}>
      <SelectTrigger>
        {value.length > 0 ? value.join(', ') : 'Select options'}
      </SelectTrigger>
      <SelectContainer showConfirmButton>
        <SelectOption value="option1">Option 1</SelectOption>
        <SelectOption value="option2">Option 2</SelectOption>
        <SelectOption value="option3">Option 3</SelectOption>
      </SelectContainer>
    </Select>
  )
}
