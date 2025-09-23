import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'

export function DevupUICard() {
  return (
    <VStack
      bg="$containerBackground"
      border="1px solid $border"
      borderRadius="20px"
      boxShadow="0 0 8px 0 var(--shadow, rgba(135, 135, 135, 0.25))"
      gap="60px"
      justifyContent="center"
      overflow="hidden"
      p="30px"
      pos="relative"
      w="400px"
    >
      <Image
        aspectRatio="1"
        bottom="-30px"
        opacity="0.2"
        pos="absolute"
        right="200px"
        src="/icons/devup-ui-card.svg"
        w="260px"
      />
      <VStack gap="8px">
        <Text color="$text" typography="h5">
          Devup-ui
        </Text>
        <Text color="$text" typography="textL">
          1.0.18
        </Text>
      </VStack>
      <VStack alignItems="flex-end" gap="20px">
        <VStack alignItems="flex-end" gap="6px" justifyContent="center">
          <Text color="$text" typography="textSbold">
            Next.js Build Time
          </Text>
          <Flex gap="10px">
            <Box
              aspectRatio="1"
              bg="#FFC100"
              maskImage="url(/icons/crown.svg)"
              maskRepeat="no-repeat"
              maskSize="contain"
              w="24px"
            />
            <Text
              backgroundClip="text"
              bg="linear-gradient(270deg, #6BB1F2 0%, #8235CA 100%)"
              color="transparent"
              typography="h4"
            >
              18.35s
            </Text>
          </Flex>
        </VStack>
        <VStack alignItems="flex-end" gap="6px" justifyContent="center">
          <Text color="$text" typography="textSbold">
            Bulid Size
          </Text>
          <Flex gap="10px">
            <Box
              aspectRatio="1"
              bg="#FFC100"
              maskImage="url(/icons/crown.svg)"
              maskRepeat="no-repeat"
              maskSize="contain"
              w="24px"
            />
            <Text
              backgroundClip="text"
              bg="linear-gradient(270deg, #6BB1F2 0%, #8235CA 100%)"
              color="transparent"
              typography="h4"
            >
              57.4MB
            </Text>
          </Flex>
        </VStack>
      </VStack>
    </VStack>
  )
}
