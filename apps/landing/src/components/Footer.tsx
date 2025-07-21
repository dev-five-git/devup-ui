import { Box, css, Flex, Image, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'

export function Footer() {
  return (
    <Box as="footer" bg="$footerBg" px="16px" py={['30px', '60px']}>
      <VStack gap={['50px', '80px']} maxW="1280px" mx="auto">
        <Flex
          flexDir={['column', 'row']}
          gap={['10px', 0]}
          justifyContent="space-between"
        >
          <VStack flex="1" gap={[1, 0]}>
            <Link
              className={css({ textDecoration: 'none' })}
              href="/docs/overview"
            >
              <Text color="$footerTitle" typography="buttonS">
                Docs
              </Text>
            </Link>
            <VStack gap="14px">
              <Link
                className={css({ textDecoration: 'none' })}
                href="/docs/overview"
              >
                <Text color="$footerText" typography="footerMenu">
                  Overview
                </Text>
              </Link>

              <Link
                className={css({ textDecoration: 'none' })}
                href="/docs/installation"
              >
                <Text color="$footerText" typography="footerMenu">
                  Installation
                </Text>
              </Link>
              <Link
                className={css({ textDecoration: 'none' })}
                href="/docs/features"
              >
                <Text color="$footerText" typography="footerMenu">
                  Features
                </Text>
              </Link>
              <Link
                className={css({ textDecoration: 'none' })}
                href="/docs/api/box"
              >
                <Text color="$footerText" typography="footerMenu">
                  API
                </Text>
              </Link>
              <Link
                className={css({ textDecoration: 'none' })}
                href="/docs/devup/devup-json"
              >
                <Text color="$footerText" typography="footerMenu">
                  Devup
                </Text>
              </Link>
            </VStack>
          </VStack>
          <VStack flex="1" gap="20px">
            <Link className={css({ textDecoration: 'none' })} href="/team">
              <Text color="$footerTitle" typography="buttonS">
                Team
              </Text>
            </Link>
          </VStack>
        </Flex>
        <Flex
          alignItems={['center', 'flex-end']}
          flexDir={['column', 'row']}
          justifyContent="space-between"
        >
          <Link href="/">
            <Image
              alt="white-logo"
              src="/white-logo.svg"
              w={['164px', '204px']}
            />
          </Link>
          <VStack
            alignItems={['center', 'flex-end']}
            gap="10px"
            pt={['20px', 0]}
          >
            <Text
              color="$footerText"
              textAlign={['center', 'right']}
              typography="small"
            >
              상호: (주)데브파이브 | 대표자명: 오정민 |{' '}
              <Box as="br" display={[null, null, 'none']} />
              사업자등록번호: 사업자등록번호: 868-86-03159
              <Box as="br" display={[null, null, 'none']} />
              주소: 경기 고양시 덕양구 마상로140번길 81 4층
            </Text>
            <Text
              color="$footerTitle"
              textAlign={['center', 'right']}
              typography="small"
            >
              Copyright © 2021-2024 데브파이브. All Rights Reserved.
            </Text>
          </VStack>
        </Flex>
      </VStack>
    </Box>
  )
}
