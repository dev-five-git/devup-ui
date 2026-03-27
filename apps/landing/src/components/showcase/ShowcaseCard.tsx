import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'

interface ShowcaseCardProps {
  name: string
  image: string
}
export function ShowcaseCard({ name, image }: ShowcaseCardProps) {
  return (
    <VStack gap="8px">
      <Image aspectRatio="1.77" h="100%" src={image} w="100%" />
      <Flex alignItems="center" gap="12px">
        <Text
          color="#000"
          flex="1"
          overflow="hidden"
          textOverflow="ellipsis"
          typography="textL"
          whiteSpace="nowrap"
        >
          {name}
        </Text>
        <Flex alignItems="center" gap="4px">
          <Text color="$caption" typography="small">
            Visit Site
          </Text>
          <Box
            bg="$text"
            boxSize="16px"
            maskImage="url(/icons/link.svg)"
            maskPos="center"
            maskRepeat="no-repeat"
            maskSize="contain"
          />
        </Flex>
      </Flex>
    </VStack>
  )
}
