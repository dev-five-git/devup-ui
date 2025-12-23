'use client'

import {
  Select,
  SelectContainer,
  SelectOption,
  SelectTrigger,
} from '@devup-ui/components'

export default function Default() {
  return (
    <Select>
      <SelectTrigger>Select an option</SelectTrigger>
      <SelectContainer>
        <SelectOption value="option1">Option 1</SelectOption>
        <SelectOption value="option2">Option 2</SelectOption>
        <SelectOption value="option3">Option 3</SelectOption>
      </SelectContainer>
    </Select>
  )
}
