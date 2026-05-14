import { Textarea } from '@devup-ui/components'

/**
 * **Error**
 * Use `error` and `errorMessage` to show validation feedback.
 */
export default function Error() {
  return (
    <Textarea
      error
      errorMessage="Please enter at least 10 characters."
      placeholder="Describe your request..."
    />
  )
}
