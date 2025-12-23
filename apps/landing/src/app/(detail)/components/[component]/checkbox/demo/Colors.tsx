'use client'

import { Checkbox } from '@devup-ui/components'

export default function Colors() {
  return (
    <Checkbox colors={{ primary: '#10B981' }} defaultChecked>
      Custom primary color
    </Checkbox>
  )
}
