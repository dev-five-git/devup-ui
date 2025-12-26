import { Toggle } from '@devup-ui/components'
import { Flex } from '@devup-ui/react'

/**
 * **Colors**
 * Pass in an object containing color tokens into `colors` prop. Customize the toggle appearance using `primary`, `bg`, `hoverBg`, `primaryHoverBg`, `disabledBg`, and more.
 */
export default function Colors() {
  return (
    <Flex gap="16px">
      <Toggle colors={{ primary: '#10B981' }} defaultValue />
      <Toggle colors={{ primary: 'orange' }} defaultValue />
      <Toggle colors={{ primary: 'steelblue' }} defaultValue />
    </Flex>
  )
}
