import { Input } from '@devup-ui/components'

/**
 * **Error**
 * Use `error` prop to display the input in an error state with a red border. Use `errorMessage` prop to show a validation message below the input.
 */
export default function Error() {
  return (
    <Input error errorMessage="This field is required" placeholder="Enter text" />
  )
}
