import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'

export function Feature() {
  return (
    <VStack alignItems="center" py="50px" w="100%">
      <VStack gap="40px" maxW="1440px" px="40px" w="100%">
        <VStack gap="16px">
          <Text color="$title" typography="h4">
            Features
          </Text>
          <Text color="$text" typography="textL">
            Devup UI offers a performance-optimized CSS-in-JS system, theme
            typing, <Box as="br" display={['none', null, 'initial']} />
            and amazing features for faster and safer development.
          </Text>
        </VStack>
        <VStack gap="16px">
          <Flex alignItems="center" gap="16px">
            <VStack
              bg="$containerBackground"
              border="1px solid $border"
              borderRadius="20px"
              boxShadow="0 4px 12px 0 rgba(135, 135, 135, 0.06)"
              flex="1"
              gap="20px"
              h="100%"
              p="30px"
            >
              <Image boxSize="32px" src="/icons/010.idea.svg" />
              <VStack gap="10px">
                <Text color="$title" typography="h6">
                  Zero Runtime
                </Text>
                <Text color="$text" typography="body">
                  A futuristic design that eliminates the root causes of
                  performance degradation.
                </Text>
              </VStack>
            </VStack>
            <VStack
              bg="$containerBackground"
              border="1px solid $border"
              borderRadius="20px"
              boxShadow="0 4px 12px 0 rgba(135, 135, 135, 0.06)"
              flex="1"
              gap="20px"
              p="30px"
            >
              <Box
                bg="#FFC738"
                boxSize="32px"
                maskImage="url(/icons/019.trophy.svg)"
                maskRepeat="no-repeat"
                maskSize="contain"
              />
              <VStack gap="10px">
                <Text color="$title" typography="h6">
                  Top Performance
                </Text>
                <Text color="$text" typography="body">
                  The fastest build speed and the smallest bundle size among
                  CSS-in-JS solutions.
                </Text>
              </VStack>
            </VStack>
          </Flex>
          <Flex alignItems="center" gap="16px">
            <VStack
              bg="$containerBackground"
              border="1px solid $border"
              borderRadius="20px"
              boxShadow="0 4px 12px 0 rgba(135, 135, 135, 0.06)"
              flex="1"
              gap="20px"
              h="100%"
              p="30px"
            >
              <Box
                bg="#F4868F"
                boxSize="32px"
                maskImage="url(/icons/021.heart.svg)"
                maskRepeat="no-repeat"
                maskSize="contain"
              />
              <VStack gap="10px">
                <Text color="$title" typography="h6">
                  Type Safety
                </Text>
                <Text color="$text" typography="body">
                  Enhanced DX with typing-based support.
                </Text>
              </VStack>
            </VStack>
            <VStack
              bg="$containerBackground"
              border="1px solid $border"
              borderRadius="20px"
              boxShadow="0 4px 12px 0 rgba(135, 135, 135, 0.06)"
              flex="1"
              gap="20px"
              h="100%"
              p="30px"
              pos="relative"
            >
              <Image boxSize="32px" src="/icons/016.notice.svg" />
              <VStack gap="10px">
                <Text color="$title" typography="h6">
                  Figma Plugin
                </Text>
                <Text color="$text" typography="body">
                  A Figma plugin enabling safer and faster development.{' '}
                </Text>
              </VStack>
              <Box pos="absolute" right="20px" top="20px">
                <FigmaButton />
              </Box>
            </VStack>
          </Flex>
        </VStack>
      </VStack>
    </VStack>
  )
}
export function Icons() {
  return (
    <Flex boxSize="24px">
      <Box
        bg="$text"
        h="16px"
        maskImage="url(/icons/Union.svg)"
        maskRepeat="no-repeat"
        maskSize="contain"
        w="16.002px"
      />
    </Flex>
  )
}
export function FigmaButton() {
  return (
    <VStack borderRadius="8px" p="8px">
      <Flex alignItems="center" gap="4px" justifyContent="flex-end">
        <Text color="$primary" typography="body">
          Go Figma Community
        </Text>
        <Box
          aspectRatio="1"
          bg="$primary"
          maskImage="url(/icons/icons.svg)"
          maskRepeat="no-repeat"
          maskSize="contain"
          w="16px"
        />
      </Flex>
    </VStack>
  )
}
