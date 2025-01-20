'use client'

import { Box, Flex } from '@devup-ui/react'
import { usePathname } from 'next/navigation'

export function HeaderWrap({ children }: { children: React.ReactNode }) {
  const path = usePathname()
  if (path !== '/') {
    return (
      <Box pos="sticky" top={0} transition="all, 0.2s" w="100%" zIndex={1}>
        <Flex
          alignItems="center"
          bg="$containerBackground"
          boxShadow="0px 2px 8px 0px var(--shadow, rgba(135, 135, 135, 0.25))"
          h="70px"
          justifyContent="space-between"
          mx="auto"
          px="40px"
        >
          {children}
        </Flex>
      </Box>
    )
  }
  return (
    <Box position="fixed" top={5} transition="all, 0.2s" w="100%">
      <Flex
        alignItems="center"
        bg="$containerBackground"
        borderRadius="16px"
        boxShadow="0px 2px 8px 0px var(--shadow, rgba(135, 135, 135, 0.25))"
        h="70px"
        justifyContent="space-between"
        maxW="1440px"
        mx="auto"
        px="40px"
      >
        {children}
      </Flex>
    </Box>
  )
}
