/**
 * ## Disabled
 * Set `disabled` prop to prevent user interaction. The input will have a muted appearance
 * and the clear button will be hidden.
 */
'use client'

import { Input } from '@devup-ui/components'

export default function Disabled() {
  return <Input disabled placeholder="Disabled input" />
}
