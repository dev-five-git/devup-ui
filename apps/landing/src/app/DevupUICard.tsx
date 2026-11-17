import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'

interface DevupUICardProps {
  version: string
  buildTime: string
  buildSize: string
  /** Whether Devup UI builds faster than every other card */
  fastest: boolean
  /** Whether Devup UI builds smaller than every other card */
  smallest: boolean
}

function Crown({ shown }: { shown: boolean }) {
  // Always laid out, so the card keeps its shape when Devup UI is not ahead.
  return (
    <Box
      aria-hidden
      aspectRatio="1"
      bg="#FFC100"
      data-benchmark="crown"
      maskImage="url(/icons/crown.svg)"
      maskRepeat="no-repeat"
      maskSize="contain"
      visibility={shown ? 'visible' : 'hidden'}
      w="24px"
    />
  )
}

export function DevupUICard({
  version,
  buildTime,
  buildSize,
  fastest,
  smallest,
}: DevupUICardProps) {
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
        <Text color="$text" data-benchmark="version" typography="textL">
          {version}
        </Text>
      </VStack>
      <VStack alignItems="flex-end" gap="20px">
        <VStack alignItems="flex-end" gap="6px" justifyContent="center">
          <Text color="$text" typography="textSbold">
            Next.js Build Time (single CSS)
          </Text>
          <Flex gap="10px">
            <Crown shown={fastest} />
            <Text
              backgroundClip="text"
              bg="linear-gradient(270deg, #6BB1F2 0%, #8235CA 100%)"
              color="transparent"
              data-benchmark="time"
              typography="h4"
            >
              {buildTime}
            </Text>
          </Flex>
        </VStack>
        <VStack alignItems="flex-end" gap="6px" justifyContent="center">
          <Text color="$text" typography="textSbold">
            Build Size
          </Text>
          <Flex gap="10px">
            <Crown shown={smallest} />
            <Text
              backgroundClip="text"
              bg="linear-gradient(270deg, #6BB1F2 0%, #8235CA 100%)"
              color="transparent"
              data-benchmark="size"
              typography="h4"
            >
              {buildSize}
            </Text>
          </Flex>
        </VStack>
      </VStack>
    </Flex>
  )
}
