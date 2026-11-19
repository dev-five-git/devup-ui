'use client'

import { Search } from '@devup-ui/components'
import { Text, VStack } from '@devup-ui/react'
import { useState } from 'react'

/**
 * **On Search**
 * Use `onSearch` to run a search when the user presses `Enter`. It receives the current query, and is not called on the `Enter` that confirms an IME composition.
 */
export default function OnSearch() {
  const [submitted, setSubmitted] = useState('')

  return (
    <VStack gap="12px">
      <Search
        aria-label="Search and press Enter"
        onSearch={setSubmitted}
        placeholder="Type and press Enter"
      />
      <Text as="p" color="$text" m="0" typography="bodyReg">
        {submitted ? `Searched for "${submitted}"` : 'Nothing searched yet'}
      </Text>
    </VStack>
  )
}
