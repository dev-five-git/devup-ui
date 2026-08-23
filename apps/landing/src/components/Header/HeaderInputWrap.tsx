'use client'
import { Box } from '@devup-ui/react'
import { usePathname, useRouter } from 'next/navigation'

import { replaceQuery } from '@/utils/use-query-param'

import { isRoot } from '../../utils/is-root'

interface HeaderInputWrapProps {
  children: React.ReactNode
}

export function HeaderInputWrap({ children }: HeaderInputWrapProps) {
  const path = usePathname()
  const root = isRoot(path)
  const router = useRouter()

  return root ? null : (
    <Box
      onClick={() => {
        replaceQuery(router, '?search=1')
      }}
    >
      {children}
    </Box>
  )
}
