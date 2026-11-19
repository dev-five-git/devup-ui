'use client'

import { Search } from '@devup-ui/components'
import { Text, VStack } from '@devup-ui/react'
import { useState } from 'react'

const FRAMEWORKS = ['Next.js', 'Vite', 'Rsbuild', 'Webpack', 'Bun']

/**
 * **Filter**
 * Use `onChange` to filter a list while the user types. Clearing the query with the clear button or `Escape` also fires `onChange`, so the list resets.
 */
export default function Filter() {
  const [query, setQuery] = useState('')
  const matches = FRAMEWORKS.filter((name) =>
    name.toLowerCase().includes(query.trim().toLowerCase()),
  )

  return (
    <VStack gap="12px">
      <Search
        aria-label="Filter frameworks"
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Filter frameworks"
        value={query}
      />
      <Text as="p" color="$text" m="0" typography="bodyReg">
        {matches.length ? matches.join(', ') : 'No match'}
      </Text>
    </VStack>
  )
}
