import { Checkbox } from '@devup-ui/components'
import { Flex } from '@devup-ui/react'

/**
 * **Colors**
 * Pass in an object containing color tokens into `colors` prop. You can customize the checkbox appearance using `primary`, `border`, `text`, `inputBg`, and `checkIcon` color values.
 */
export default function Colors() {
  return (
    <Flex flexWrap="wrap" gap="16px">
      <Checkbox colors={{ primary: '#10B981' }} defaultChecked>
        Green
      </Checkbox>
      <Checkbox colors={{ primary: 'orange' }} defaultChecked>
        Orange
      </Checkbox>
      <Checkbox colors={{ primary: 'steelblue' }} defaultChecked>
        Steel Blue
      </Checkbox>
    </Flex>
  )
}
