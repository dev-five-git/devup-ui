import {
  Select,
  SelectContainer,
  SelectOption,
  SelectTrigger,
} from '@devup-ui/components'

/**
 * **Default**
 * Compound component with trigger and dropdown. Use `SelectTrigger` to create the button, `SelectContainer` for the dropdown, and `SelectOption` for each option.
 */
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
