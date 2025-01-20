'use client'

import { Box, Flex } from '@devup-ui/react'
import { usePathname } from 'next/navigation'

export function HeaderWrap({ children }: { children: React.ReactNode }) {
  const path = usePathname()
  const isRoot = path === '/'
  return (
    <Box
      pos={isRoot ? 'fixed' : 'sticky'}
      top={isRoot ? 5 : 0}
      transition="all, 0.2s"
      w="100%"
      zIndex={1}
    >
      <Flex
        alignItems="center"
        bg="$containerBackground"
        borderRadius={isRoot ? '16px' : undefined}
        boxShadow="0px 2px 8px 0px var(--shadow, rgba(135, 135, 135, 0.25))"
        h="70px"
        justifyContent="space-between"
        maxW={isRoot ? '1440px' : '100%'}
        mx="auto"
        px="40px"
      >
        {children}
      </Flex>
    </Box>
  )
}
