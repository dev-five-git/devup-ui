'use client'

import { Button, ProgressBar } from '@devup-ui/components'
import { Flex, VStack } from '@devup-ui/react'
import { useState } from 'react'

/**
 * **Show Value**
 * Use `showValue` to print the progress above the bar: a percent for a bar, `done/total` for steps. Values outside `0` to `max` are clamped.
 */
export default function ShowValue() {
  const [value, setValue] = useState(30)

  return (
    <VStack gap="16px">
      <ProgressBar aria-label="Download progress" showValue value={value} />
      <Flex gap="8px">
        <Button onClick={() => setValue((v) => Math.max(v - 10, 0))}>
          -10
        </Button>
        <Button onClick={() => setValue((v) => Math.min(v + 10, 100))}>
          +10
        </Button>
      </Flex>
    </VStack>
  )
}
