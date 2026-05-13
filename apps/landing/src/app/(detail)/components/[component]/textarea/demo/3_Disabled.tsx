import { Textarea } from '@devup-ui/components'

/**
 * **Disabled**
 * Disabled textareas keep the value visible while preventing edits.
 */
export default function Disabled() {
  return (
    <Textarea
      defaultValue="This message is read-only."
      disabled
      placeholder="Disabled textarea"
    />
  )
}
