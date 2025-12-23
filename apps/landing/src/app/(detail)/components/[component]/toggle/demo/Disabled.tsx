'use client'

import { Flex } from '@devup-ui/react'
import { Toggle } from '@devup-ui/components'

export default function Disabled() {
  return (
    <Flex gap="16px">
      <Toggle disabled />
      <Toggle defaultValue disabled />
    </Flex>
  )
}
