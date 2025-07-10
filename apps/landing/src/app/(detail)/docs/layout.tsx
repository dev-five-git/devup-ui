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
      <Flex maxW="1920px" minH="calc(100vh - 500px)" mx="auto">
        <Box display={['none', null, 'initial']} p="20px 16px" w="220px">
          <Box pos="sticky" top={['70px', null, '90px']}>
            <LeftMenu />
          </Box>
        </Box>
        <Box
          className="markdown-body"
          flex={1}
          px={['20px', null, '60px']}
          py={['20px', null, '40px']}
          w="100%"
        >
          {children}
        </Box>
        <Box display={['none', null, null, null, null, 'initial']}>
          <Box pos="sticky" top={['50px', null, '70px']}>
            <RightIndex />
          </Box>
        </Box>
      </Flex>
    </>
  )
}
