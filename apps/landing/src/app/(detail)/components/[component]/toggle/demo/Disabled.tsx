import { Toggle } from '@devup-ui/components'
import { Flex } from '@devup-ui/react'

/**
 * **Disabled**
 * Use `disabled` prop to prevent user interaction. The toggle will have a muted appearance and cannot be toggled.
 */
export default function Disabled() {
  return (
    <Flex gap="16px">
      <Toggle disabled />
      <Toggle defaultValue disabled />
    </Flex>
  )
}
