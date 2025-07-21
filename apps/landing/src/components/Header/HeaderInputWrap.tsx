'use client'
import { Box } from '@devup-ui/react'
import { usePathname, useRouter } from 'next/navigation'

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
        router.replace('?search=1')
      }}
    >
      {children}
    </Box>
  )
}
