import { Select } from '@devup-ui/components'

/**
 * **Options**
 * Use the `options` prop as a shorthand to define options without compound components. Each option can have `label`, `value`, `disabled`, and `onClick` properties.
 */
export default function WithOptions() {
  return (
    <Select
      options={[
        { label: 'Option 1', value: 'option1' },
        { label: 'Option 2', value: 'option2' },
        { label: 'Option 3', value: 'option3', disabled: true },
      ]}
    >
      Select with options prop
    </Select>
  )
}
