/**
 * ## Colors
 * Pass an object containing color tokens into `colors` prop. Customize the toggle appearance
 * using `primary`, `bg`, `hoverBg`, `primaryHoverBg`, `disabledBg`, and more.
 */
'use client'

import { Toggle } from '@devup-ui/components'

export default function Colors() {
  return <Toggle colors={{ primary: '#10B981' }} defaultValue />
}
