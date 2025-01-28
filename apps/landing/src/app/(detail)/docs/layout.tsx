import { Box, Flex } from '@devup-ui/react'

import { LeftMenu } from './LeftMenu'
import { RightIndex } from './RightIndex'

export default function DetailLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <>
      <Flex maxW="1440px" mx="auto">
        <Box p="20px 16px" w="220px">
          <LeftMenu />
        </Box>
        <Box className="markdown-body" flex={1} px="60px" py="40px">
          {children}
        </Box>
        <RightIndex />
      </Flex>
    </>
  )
}
