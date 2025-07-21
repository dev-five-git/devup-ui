'use client'

import { Box, Flex } from '@devup-ui/react'
import { usePathname } from 'next/navigation'

import { isRoot } from '../../utils/is-root'

export function HeaderWrap({ children }: { children: React.ReactNode }) {
  const path = usePathname()
  const root = isRoot(path)
  return (
    <Box
      pos={root ? 'fixed' : 'sticky'}
      pt={root ? [null, null, 5] : undefined}
      px={root ? [null, null, 4] : undefined}
      top="0"
      transition="all, 0.2s"
      w="100%"
      zIndex={2}
    >
      <HeaderWrapInner>{children}</HeaderWrapInner>
    </Box>
  )
}

function HeaderWrapInner({ children }: { children: React.ReactNode }) {
  const path = usePathname()
  const root = isRoot(path)
  return (
    <Flex
      alignItems="center"
      bg="$containerBackground"
      borderRadius={root ? [null, null, '16px'] : undefined}
      boxShadow={
        root
          ? '0px 2px 8px 0px var(--shadow, rgba(135, 135, 135, 0.25))'
          : '0px 1px 1px 0px var(--shadow, rgba(135, 135, 135, 0.25))'
      }
      h={['50px', null, '70px']}
      justifyContent="space-between"
      maxW={root ? '1440px' : '100%'}
      mx="auto"
      px={[null, 5, '20px']}
    >
      {children}
    </Flex>
  )
}
