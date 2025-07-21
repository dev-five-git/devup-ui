'use client'

import { css, Flex, Image, Text } from '@devup-ui/react'
import Link from 'next/link'

export default function SponsorButton() {
  return (
    <Link
      className={css({
        textDecoration: 'none',
      })}
      href="https://github.com/sponsors/dev-five-git"
      target="_blank"
    >
      <Flex
        _active={{
          bg: '$menuActive',
        }}
        _hover={{
          bg: '$menuHover',
        }}
        alignItems="center"
        bg="$containerBackground"
        border="1px solid $imageBorder"
        borderRadius="12px"
        gap="10px"
        pl="16px"
        pr="20px"
        py="10px"
        role="group"
        transition="all 0.1s ease-in-out"
      >
        <Image
          _groupHover={{
            transform: 'scale(1.1)',
          }}
          aspectRatio="1"
          boxSize="24px"
          src="/icons/solar_heart-bold.svg"
          transition="transform 0.2s ease-in-out"
        />
        <Text color="$text" textAlign="center" typography="buttonLsemiB">
          Sponsor
        </Text>
      </Flex>
    </Link>
  )
}
