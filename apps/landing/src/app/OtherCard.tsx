import { css, Flex, Text, VStack } from '@devup-ui/react'
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
    <Flex
      aspectRatio={[null, null, null, null, '1']}
      bg="$cardBg"
      borderRadius="20px"
      flexDir={['row', null, 'column']}
      gap="40px"
      justifyContent="space-between"
      minW={[null, null, '240px', null, '300px']}
      p={[6, null, '30px']}
    >
      <VStack gap="8px">
        <Link
          className={css({
            textDecoration: 'none',
          })}
          href={url}
          target="_blank"
        >
          <Text color="$captionBold" typography="h6">
            {title}
          </Text>
        </Link>
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
    </Flex>
  )
}
