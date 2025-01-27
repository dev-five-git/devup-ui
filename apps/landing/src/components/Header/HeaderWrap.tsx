'use client'

import { Box, Flex } from '@devup-ui/react'
import { usePathname, useSearchParams } from 'next/navigation'

export function HeaderWrap({ children }: { children: React.ReactNode }) {
  const path = usePathname()
  const isRoot = path === '/'
  const menu = useSearchParams().get('menu') === '1'
  return (
    <Box
      pos={isRoot ? 'fixed' : 'sticky'}
      pt={isRoot ? [null, null, 5] : undefined}
      px={isRoot ? [null, null, 4] : undefined}
      top="0"
      transition="all, 0.2s"
      w="100%"
      zIndex={1}
    >
      <Flex
        alignItems="center"
        bg="$containerBackground"
        borderRadius={isRoot ? [null, null, '16px'] : undefined}
        boxShadow="0px 2px 8px 0px var(--shadow, rgba(135, 135, 135, 0.25))"
        h={['50px', null, '70px']}
        justifyContent="space-between"
        maxW={isRoot ? '1440px' : '100%'}
        mx="auto"
        pl={[menu ? null : 4, 5, '40px']}
        pr={[null, 5, '40px']}
      >
        {children}
      </Flex>
    </Box>
  )
}
