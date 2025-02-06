'use client'

import { Box } from '@devup-ui/react'
import { useRouter, useSearchParams } from 'next/navigation'

export function SearchModalWrap({ children }: { children?: React.ReactNode }) {
  const search = useSearchParams().get('search')
  const router = useRouter()

  return search !== '1' ? null : (
    <Box
      bg="rgba(0, 0, 0, 0.70)"
      boxSize="100%"
      onClick={(event) => {
        if (event.target === event.currentTarget) router.replace('?')
      }}
      pos="fixed"
      zIndex={10000}
    >
      {children}
    </Box>
  )
}
