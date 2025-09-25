import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'

export function DevupUICard() {
  return (
    <Flex
      bg="$containerBackground"
      border="1px solid $border"
      borderRadius="20px"
      boxShadow="0 0 8px 0 var(--shadow, rgba(135, 135, 135, 0.25))"
      flexDir={['row', null, null, null, 'column']}
      gap="60px"
      justifyContent={['space-between', null, null, null, 'center']}
      mx={[4, null, '40px', null, 0]}
      overflow="hidden"
      p={['24px', null, '30px']}
      pos="relative"
      w={['100%', null, null, null, '400px']}
    >
      <Image
        aspectRatio="1"
        bottom="-30px"
        left="-50px"
        opacity="0.2"
        pos="absolute"
        src="/icons/devup-ui-card.svg"
        w="260px"
      />
      <VStack gap="8px">
        <Text color="$text" typography="h5">
          Devup UI
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
              57.40MB
            </Text>
          </Flex>
        </VStack>
      </VStack>
    </Flex>
  )
}
