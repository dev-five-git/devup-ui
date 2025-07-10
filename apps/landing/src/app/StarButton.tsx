import { Center, css, Flex, Image, Text } from '@devup-ui/react'
import Link from 'next/link'

export default async function StarButton() {
  const res = await fetch('https://api.github.com/repos/dev-five-git/devup-ui')
  const data = await res.json()
  const starCount = data.stargazers_count

  return (
    <Link
      className={css({
        textDecoration: 'none',
      })}
      href="https://github.com/dev-five-git/devup-ui"
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
        cursor="pointer"
        h="100%"
        role="group"
        transition="all 0.1s ease-in-out"
      >
        <Flex
          alignItems="center"
          borderRadius="12px 0 0 12px"
          gap="10px"
          pl="16px"
          pr="20px"
          py="10px"
        >
          <Image
            _groupHover={{
              transform: 'scale(1.1)',
            }}
            aspectRatio="1"
            boxSize="24px"
            src="/icons/solar_star-bold.svg"
            transition="transform 0.2s ease-in-out"
          />
          <Text color="$text" textAlign="center" typography="buttonLsemiB">
            Star
          </Text>
        </Flex>
        <Center
          bg="$starBg"
          borderLeft="1px solid $imageBorder"
          borderRadius="0 12px 12px 0"
          h="100%"
          px="16px"
        >
          <Text color="$primary" textAlign="center" typography="buttonLsemiB">
            {starCount}
          </Text>
        </Center>
      </Flex>
    </Link>
  )
}
