/**
 * ## Colors
 * Pass an object containing color tokens into `colors` prop. You can customize the checkbox
 * appearance using `primary`, `border`, `text`, `inputBg`, and `checkIcon` color values.
 */
'use client'

import { Checkbox } from '@devup-ui/components'

export default function Colors() {
  return (
    <Checkbox colors={{ primary: '#10B981' }} defaultChecked>
      Custom primary color
    </Checkbox>
  )
}
