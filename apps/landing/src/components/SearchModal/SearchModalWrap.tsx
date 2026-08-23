'use client'

import { Box } from '@devup-ui/react'
import { useRouter } from 'next/navigation'

import { replaceQuery, useQueryParam } from '@/utils/use-query-param'

export function SearchModalWrap({ children }: { children?: React.ReactNode }) {
  const search = useQueryParam('search')
  const router = useRouter()

  return search !== '1' ? null : (
    <Box
      bg="rgba(0, 0, 0, 0.70)"
      boxSize="100%"
      onClick={(event) => {
        if (event.target === event.currentTarget) replaceQuery(router, '?')
      }}
      pos="fixed"
      zIndex={10000}
    >
      {children}
    </Box>
  )
}
