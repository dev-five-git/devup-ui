import { Box, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'

interface OtherCardProps {
  title: string
  version: string
  buildTime: string
  buildSize: string
  url: string
}
export function OtherCard({
  title,
  version,
  buildTime,
  buildSize,
  url,
}: OtherCardProps) {
  return (
    <VStack
      aspectRatio="1"
      bg="$cardBg"
      borderRadius="20px"
      gap="40px"
      justifyContent="center"
      minW="300px"
      p="30px"
      w="300px"
    >
      <VStack gap="8px">
        <Box
          as={Link}
          props={{
            href: url,
            target: '_blank',
            children: (
              <Text color="$captionBold" typography="h6">
                {title}
              </Text>
            ),
          }}
          textDecoration="none"
        ></Box>
        <Text color="$captionBold" typography="textL">
          {version}
        </Text>
      </VStack>
      <VStack alignItems="flex-end" gap="20px">
        <VStack alignItems="flex-end" gap="6px" justifyContent="center">
          <Text color="$captionBold" typography="textSbold">
            Bulid Time
          </Text>
          <Text color="$caption" typography="h5">
            {buildTime}
          </Text>
        </VStack>
        <VStack alignItems="flex-end" gap="6px" justifyContent="center">
          <Text color="$captionBold" typography="textSbold">
            Bulid Size
          </Text>
          <Text color="$caption" typography="h5">
            {buildSize}
          </Text>
        </VStack>
      </VStack>
    </VStack>
  )
}
