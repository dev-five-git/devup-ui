import { Box, Center, Flex, Image, Text, VStack } from '@devup-ui/react'

export function Footer() {
  return (
    <Box bg="$background" pt={['30px', '60px']}>
      <Center as="footer" bg="$footerBg" px="30px" py="60px">
        <Flex
          flexDir={['column', 'row']}
          gap={['20px', '40px']}
          justifyContent="space-between"
          maxW="1440px"
          w="100%"
        >
          <Image h="43px" objectFit="contain" src="/white-logo.svg" w="205px" />
          <VStack alignItems="flex-end" gap="10px" justifyContent="center">
            <Text
              color="$footerText"
              textAlign="right"
              typography="small"
              wordBreak="keep-all"
            >
              상호: (주)데브파이브 | 대표자명: 오정민 | 사업자등록번호:
              868-86-03159
              <Box as="br" display={['none', null, 'initial']} />
              주소: 경기 고양시 덕양구 마상로140번길 81 4층
            </Text>
            <Text color="$footerTitle" typography="small">
              Copyright © 2021-2024 데브파이브. All Rights Reserved.
            </Text>
          </VStack>
        </Flex>
      </Center>
    </Box>
  )
}
