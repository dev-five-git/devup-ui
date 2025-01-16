import { Flex, Image, Text, VStack } from '@devup-ui/react'

import { IMAGE_PREFIX } from '../constants'

export function Footer() {
  return (
    <VStack bg="$footerBg" gap="80px" p="60px 320px">
      <Flex justifyContent="space-between">
        <VStack flex="1 0 0" gap="20px" minWidth="240px">
          <Text color="$footerTitle" typography="buttonS">
            메뉴 타이틀 1
          </Text>
          <VStack gap="14px">
            <Text color="$footerText" typography="footerMenu">
              상세 메뉴 1
            </Text>
            <Text color="$footerText" typography="footerMenu">
              상세 메뉴 2
            </Text>
            <Text color="$footerText" typography="footerMenu">
              상세 메뉴 3
            </Text>
            <Text color="$footerText" typography="footerMenu">
              상세 메뉴 4
            </Text>
            <Text color="$footerText" typography="footerMenu">
              상세 메뉴 5
            </Text>
          </VStack>
        </VStack>
        <VStack flex="1 0 0" gap="20px" minWidth="240px">
          <Text color="$footerTitle" typography="buttonS">
            메뉴 타이틀 2
          </Text>
          <Text color="$footerText" typography="footerMenu">
            상세 메뉴 1
          </Text>
          <Text color="$footerText" typography="footerMenu">
            상세 메뉴 2
          </Text>
        </VStack>
        <VStack flex="1 0 0" gap="20px" minWidth="240px">
          <Text color="$footerTitle" typography="buttonS">
            메뉴 타이틀 3
          </Text>
          <Text color="$footerText" typography="footerMenu">
            상세 메뉴 1
          </Text>
          <Text color="$footerText" typography="footerMenu">
            상세 메뉴 2
          </Text>
          <Text color="$footerText" typography="footerMenu">
            상세 메뉴 3
          </Text>
          <Text color="$footerText" typography="footerMenu">
            상세 메뉴 4
          </Text>
        </VStack>
      </Flex>
      <Flex alignItems="flex-end" justifyContent="space-between">
        <Flex alignItems="flex-end">
          <Image alt="white-logo" src={IMAGE_PREFIX + '/white-logo.svg'} />
        </Flex>
        <VStack alignItems="flex-end" gap="10px">
          <Text color="$footerText" textAlign="right" typography="small">
            상호: (주)데브파이브 | 대표자명: 오정민 | 사업자등록번호:
            868-86-03159 주소: 경기 고양시 덕양구 마상로140번길 81 4층
          </Text>
          <Text color="$footerTitle" textAlign="right" typography="small">
            Copyright © 2021-2024 데브파이브. All Rights Reserved.
          </Text>
        </VStack>
      </Flex>
    </VStack>
  )
}
