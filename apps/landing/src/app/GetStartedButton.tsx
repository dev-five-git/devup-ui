import { Box, css, Flex, Image, Text } from '@devup-ui/react'
import Link from 'next/link'

export function GetStartedButton() {
  return (
    <Link
      className={css({
        textDecoration: 'none',
      })}
      href="/docs/overview"
    >
      <Flex
        _active={{
          bg: '$negativeBase',
        }}
        _hover={{
          bg: '$title',
        }}
        alignItems="center"
        bg="$text"
        borderRadius="100px"
        gap="20px"
        p="16px 40px"
        role="group"
        w="fit-content"
      >
        <Box
          _groupActive={{
            bg: '$third',
          }}
          _groupHover={{
            bg: '$primary',
          }}
          bg="$secondary"
          borderRadius="100%"
          boxSize="10px"
        />
        <Flex alignItems="center" gap="10px">
          <Text color="$base" typography="buttonL">
            Get started
          </Text>
          <Image bg="$base" boxSize="24px" maskImage="url(/arrow.svg)" />
        </Flex>
      </Flex>
    </Link>
  )
}
