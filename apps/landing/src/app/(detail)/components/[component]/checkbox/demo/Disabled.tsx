import { Checkbox } from '@devup-ui/components'
import { VStack } from '@devup-ui/react'

/**
 * **Disabled**
 * Use `disabled` prop to prevent user interaction. The checkbox will have a muted appearance and cannot be toggled.
 */
export default function Disabled() {
  return (
    <VStack gap="8px">
      <Checkbox disabled>Disabled unchecked</Checkbox>
      <Checkbox defaultChecked disabled>
        Disabled checked
      </Checkbox>
    </VStack>
  )
}
