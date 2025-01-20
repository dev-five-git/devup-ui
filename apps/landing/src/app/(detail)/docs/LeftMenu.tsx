import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'

export function LeftMenu() {
  return (
    <VStack gap="6px" h="1008px" p="20px 16px" w="220px">
      <Flex alignItems="center" borderRadius="6px" p="6px 10px">
        <Text flex="1" opacity="0.8" typography="buttonSmid">
          개요
        </Text>
      </Flex>
      <Flex
        alignItems="center"
        bg="$menuActive"
        borderRadius="6px"
        gap="10px"
        p="6px 10px"
      >
        <Box bg="$primary" boxSize="8px" />
        <Text flex="1" typography="buttonS">
          설치
        </Text>
      </Flex>
      <Flex alignItems="center" borderRadius="6px" gap="10px" p="6px 10px">
        <Text flex="1" opacity="0.8" typography="buttonSmid">
          개념
        </Text>
        <Image boxSize="16px" src="/menu-arrow.svg" />
      </Flex>
      <Flex gap="8px">
        <Box borderColor="$border" w="10px" />
        <VStack flex="1" gap="6px">
          <Flex alignItems="center" borderRadius="6px" p="6px 10px">
            <Text flex="1" opacity="0.8" typography="buttonSmid">
              유틸리티 퍼스트
            </Text>
          </Flex>
          <Flex alignItems="center" borderRadius="6px" p="6px 10px">
            <Text flex="1" opacity="0.8" typography="buttonSmid">
              하이브리드 접근 방식
            </Text>
          </Flex>
          <Flex alignItems="center" borderRadius="6px" p="6px 10px">
            <Text flex="1" opacity="0.8" typography="buttonSmid">
              헤드리스 컴포넌트
            </Text>
          </Flex>
        </VStack>
      </Flex>
      <Flex alignItems="center" borderRadius="6px" gap="10px" p="6px 10px">
        <Text flex="1" opacity="0.8" typography="buttonSmid">
          구성 요소
        </Text>
        <Image boxSize="16px" src="/menu-arrow.svg" />
      </Flex>
      <Flex alignItems="center" borderRadius="6px" gap="10px" p="6px 10px">
        <Text flex="1" opacity="0.8" typography="buttonSmid">
          API
        </Text>
        <Image boxSize="16px" src="/menu-arrow.svg" />
      </Flex>
      <Flex gap="8px">
        <Box borderColor="$border" w="10px" />
        <VStack flex="1" gap="6px">
          <Flex alignItems="center" borderRadius="6px" p="6px 10px">
            <Text flex="1" opacity="0.8" typography="buttonSmid">
              스타일이 지정됨
            </Text>
          </Flex>
          <Flex alignItems="center" borderRadius="6px" p="6px 10px">
            <Text flex="1" opacity="0.8" typography="buttonSmid">
              CSS
            </Text>
          </Flex>
        </VStack>
      </Flex>
      <Flex alignItems="center" borderRadius="6px" gap="10px" p="6px 10px">
        <Text flex="1" opacity="0.8" typography="buttonSmid">
          테마
        </Text>
        <Image boxSize="16px" src="/menu-arrow.svg" />
      </Flex>
      <Flex gap="8px">
        <Box borderColor="$border" w="10px" />
        <VStack flex="1" gap="6px">
          <Flex alignItems="center" borderRadius="6px" p="6px 10px">
            <Text flex="1" opacity="0.8" typography="buttonSmid">
              테마 사용자 정의
            </Text>
          </Flex>
          <Flex alignItems="center" borderRadius="6px" p="6px 10px">
            <Text flex="1" opacity="0.8" typography="buttonSmid">
              테마 토큰
            </Text>
          </Flex>
          <Flex alignItems="center" borderRadius="6px" p="6px 10px">
            <Text flex="1" opacity="0.8" typography="buttonSmid">
              중단점
            </Text>
          </Flex>
          <Flex alignItems="center" borderRadius="6px" p="6px 10px">
            <Text flex="1" opacity="0.8" typography="buttonSmid">
              구성 요소 테마
            </Text>
          </Flex>
        </VStack>
      </Flex>
    </VStack>
  )
}
