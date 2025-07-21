'use client'
import { useRouter } from 'next/navigation'

import { HeaderInput } from '../Header/HeaderInput'

export function SearchInput() {
  const router = useRouter()
  return (
    <HeaderInput
      ref={(input) => {
        if (input) {
          input.focus()
          const query = new URLSearchParams(window.location.search).get('query')
          if (query) input.value = query
        }
      }}
      onChange={(e) => {
        router.replace('?search=1&query=' + e.target.value)
      }}
    />
  )
}
