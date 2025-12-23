'use client'

import { VStack } from '@devup-ui/react'
import { Checkbox } from '@devup-ui/components'

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
